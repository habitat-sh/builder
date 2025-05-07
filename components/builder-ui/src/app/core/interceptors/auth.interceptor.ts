import { Injectable } from '@angular/core';
import {
  HttpEvent,
  HttpInterceptor,
  HttpHandler,
  HttpRequest,
  HttpErrorResponse
} from '@angular/common/http';
import { Observable, throwError, BehaviorSubject, of } from 'rxjs';
import { catchError, switchMap, filter, take, finalize } from 'rxjs/operators';
import { AuthService } from '../services/auth.service';

/**
 * Interceptor for handling authentication tokens and token refresh
 */
@Injectable()
export class AuthInterceptor implements HttpInterceptor {
  private isRefreshing = false;
  private refreshTokenSubject: BehaviorSubject<string | null> = new BehaviorSubject<string | null>(null);

  constructor(private authService: AuthService) {}

  intercept(req: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Skip adding token for auth endpoints
    if (this.isAuthEndpoint(req.url)) {
      return next.handle(req);
    }

    // Add auth token to request if available
    const token = this.authService.token();
    if (token) {
      req = this.addTokenToRequest(req, token);
    }

    // Process the request and handle auth errors
    return next.handle(req).pipe(
      catchError(error => {
        if (error instanceof HttpErrorResponse && error.status === 401) {
          return this.handle401Error(req, next);
        }
        
        return throwError(() => error);
      })
    );
  }

  /**
   * Add authentication token to an HTTP request
   */
  private addTokenToRequest(req: HttpRequest<any>, token: string): HttpRequest<any> {
    return req.clone({
      setHeaders: {
        Authorization: `Bearer ${token}`
      }
    });
  }

  /**
   * Handle 401 Unauthorized errors, potentially refreshing the token
   */
  private handle401Error(request: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Don't try to refresh if this is an auth endpoint
    if (this.isAuthEndpoint(request.url)) {
      return throwError(() => new Error('Authentication failed'));
    }

    if (!this.isRefreshing) {
      this.isRefreshing = true;
      this.refreshTokenSubject.next(null);

      // Attempt to refresh the token
      return this.authService.refreshToken().pipe(
        switchMap(success => {
          this.isRefreshing = false;
          
          if (success) {
            const newToken = this.authService.token();
            this.refreshTokenSubject.next(newToken);
            
            if (newToken) {
              // Retry the original request with new token
              return next.handle(this.addTokenToRequest(request, newToken));
            }
          }
          
          // If refresh failed, redirect to login
          this.authService.logout();
          return throwError(() => new Error('Session expired'));
        }),
        catchError(error => {
          this.isRefreshing = false;
          this.authService.logout();
          return throwError(() => error);
        }),
        finalize(() => {
          this.isRefreshing = false;
        })
      );
    } else {
      // Wait for token refresh to complete and retry with new token
      return this.refreshTokenSubject.pipe(
        filter(token => token !== null),
        take(1),
        switchMap(token => {
          if (token) {
            return next.handle(this.addTokenToRequest(request, token));
          }
          
          return throwError(() => new Error('Authentication failed'));
        })
      );
    }
  }

  /**
   * Check if URL is an authentication endpoint that shouldn't have token added
   */
  private isAuthEndpoint(url: string): boolean {
    const authPaths = [
      '/auth/login',
      '/auth/register',
      '/auth/oauth',
      '/auth/refresh'
    ];
    
    return authPaths.some(path => url.includes(path));
  }
}
