import {
  HttpEvent,
  HttpHandlerFn,
  HttpInterceptorFn,
  HttpRequest,
  HttpErrorResponse
} from '@angular/common/http';
import { inject } from '@angular/core';
import { Observable, throwError, BehaviorSubject } from 'rxjs';
import { catchError, switchMap, filter, take, finalize } from 'rxjs/operators';
import { AuthService } from '../services/auth.service';

// Maintain state between requests
let isRefreshing = false;
const refreshTokenSubject = new BehaviorSubject<string | null>(null);

/**
 * Interceptor for handling authentication tokens and token refresh
 */
export const AuthInterceptor: HttpInterceptorFn = (req: HttpRequest<unknown>, next: HttpHandlerFn): Observable<HttpEvent<unknown>> => {
  const authService = inject(AuthService);
  
  // Skip adding token for auth endpoints
  if (isAuthEndpoint(req.url)) {
    return next(req);
  }

  // Add auth token to request if available
  const token = authService.token();
  if (token) {
    req = addTokenToRequest(req, token);
  }

  return next(req).pipe(
    catchError(error => {
      if (error instanceof HttpErrorResponse && error.status === 401) {
        return handle401Error(req, next, authService);
      }
      return throwError(() => error);
    })
  );
};

/**
 * Check if a URL is an auth endpoint that should bypass token handling
 */
function isAuthEndpoint(url: string): boolean {
  const authPaths = [
    '/v1/signin',
    '/v1/authenticate',
    '/v1/users/authn', 
    '/v1/users/auth_token',
    '/v1/users/refresh_token',
    '/auth/login',
    '/auth/callback',
    '/auth/refresh',
    '/oauth/github',
    '/oauth/token'
  ];
  
  // Match either exact paths or paths that contain these patterns
  return authPaths.some(path => url.includes(path));
}

/**
 * Add authorization token to a request
 */
function addTokenToRequest(req: HttpRequest<unknown>, token: string): HttpRequest<unknown> {
  return req.clone({
    setHeaders: {
      Authorization: `Bearer ${token}`
    }
  });
}

/**
 * Handle 401 Unauthorized errors and attempt token refresh
 */
function handle401Error(
  request: HttpRequest<unknown>, 
  next: HttpHandlerFn,
  authService: AuthService
): Observable<HttpEvent<unknown>> {
  // Don't try to refresh if this is an auth endpoint
  if (isAuthEndpoint(request.url)) {
    return throwError(() => new Error('Authentication failed'));
  }

  if (!isRefreshing) {
    isRefreshing = true;
    refreshTokenSubject.next(null);

    return authService.refreshToken().pipe(
      switchMap(success => {
        isRefreshing = false;
        const newToken = authService.token();
        
        if (newToken) {
          refreshTokenSubject.next(newToken);
          return next(addTokenToRequest(request, newToken));
        }
        
        authService.logout();
        return throwError(() => new Error('Token refresh failed'));
      }),
      catchError(error => {
        isRefreshing = false;
        authService.logout();
        return throwError(() => error);
      })
    );
  }
  
  // Wait until token is refreshed and retry with new token
  return refreshTokenSubject.pipe(
    filter(token => token !== null),
    take(1),
    switchMap(token => {
      if (token) {
        return next(addTokenToRequest(request, token));
      }
      return throwError(() => new Error('Token refresh failed'));
    })
  );
}
