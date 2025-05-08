import {
  HttpEvent,
  HttpHandlerFn,
  HttpInterceptorFn,
  HttpRequest,
  HttpResponse
} from '@angular/common/http';
import { inject } from '@angular/core';
import { Observable } from 'rxjs';
import { tap, finalize } from 'rxjs/operators';
import { LoadingService } from '../services/loading.service';

// Track the number of active requests
let totalRequests = 0;

/**
 * Interceptor for showing loading indicator during HTTP requests
 */
export const LoadingInterceptor: HttpInterceptorFn = (request: HttpRequest<unknown>, next: HttpHandlerFn): Observable<HttpEvent<unknown>> => {
  const loadingService = inject(LoadingService);
  
  // Don't show loader for certain requests
  if (shouldSkipLoading(request)) {
    return next(request);
  }

  totalRequests++;
  loadingService.start();

  return next(request).pipe(
    tap(event => {
      // If the response is successful, check if it's the final response
      if (event instanceof HttpResponse) {
        if (totalRequests <= 1) {
          loadingService.stop();
        }
      }
    }),
    finalize(() => {
      totalRequests--;
      
      if (totalRequests === 0) {
        loadingService.stop();
      }
    })
  );
};

/**
 * Check if loading indicator should be skipped for this request
 */
function shouldSkipLoading(request: HttpRequest<unknown>): boolean {
  // Skip loading indicator for polling requests
  const isPollRequest = request.headers.has('x-skip-loading') 
    || request.url.includes('/status') 
    || request.url.includes('/health');
  
  // Skip loading indicator for background requests
  const isBackgroundRequest = request.headers.has('x-background-request');
  
  return isPollRequest || isBackgroundRequest;
}
