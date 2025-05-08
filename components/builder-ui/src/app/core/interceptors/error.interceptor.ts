import {
  HttpEvent,
  HttpHandlerFn,
  HttpInterceptorFn,
  HttpRequest,
  HttpErrorResponse
} from '@angular/common/http';
import { inject } from '@angular/core';
import { Observable, throwError } from 'rxjs';
import { catchError } from 'rxjs/operators';
import { NotificationService } from '../services/notification.service';
import { environment } from '../../../environments/environment';

/**
 * Interceptor for handling HTTP errors and displaying appropriate notifications
 */
export const ErrorInterceptor: HttpInterceptorFn = (req: HttpRequest<unknown>, next: HttpHandlerFn): Observable<HttpEvent<unknown>> => {
  const notificationService = inject(NotificationService);
  
  return next(req).pipe(
    catchError((error: HttpErrorResponse) => {
      // Don't show notification for 401 errors (handled by auth interceptor)
      if (error.status === 401) {
        return throwError(() => error);
      }

      // Extract error message from response
      const errorMessage = getErrorMessage(error);
      
      // Show notification for server errors
      if (error.status >= 500) {
        notificationService.error(`Server error: ${errorMessage}`);
      } 
      // Show notification for client errors (except 401)
      else if (error.status >= 400) {
        notificationService.warning(`Request error: ${errorMessage}`);
      }
      // Show notification for network errors
      else if (error.status === 0) {
        notificationService.error('Network error: Could not connect to server');
      }
      
      // Log detailed error in development mode
      if (!environment.production) {
        console.error('HTTP Error:', error);
      }
      
      return throwError(() => error);
    })
  );
};

/**
 * Extract error message from an HttpErrorResponse
 */
function getErrorMessage(error: HttpErrorResponse): string {
  // Client-side error
  if (error.error instanceof ErrorEvent) {
    return error.error.message;
  }
  
  // Server-side error
  if (error.error && typeof error.error === 'object') {
    // Check common API error response formats
    if (error.error.message) {
      return error.error.message;
    }
    
    if (error.error.error) {
      return typeof error.error.error === 'string' 
        ? error.error.error 
        : JSON.stringify(error.error.error);
    }
    
    if (error.error.detail) {
      return error.error.detail;
    }
  }
  
  // Fallback to status text
  return error.statusText || 'Unknown error';
}
