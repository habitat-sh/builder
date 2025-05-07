import { Injectable } from '@angular/core';
import {
  HttpEvent,
  HttpInterceptor,
  HttpHandler,
  HttpRequest,
  HttpResponse
} from '@angular/common/http';
import { Observable } from 'rxjs';
import { tap, finalize } from 'rxjs/operators';
import { LoadingService } from '../services/loading.service';

/**
 * Interceptor for showing loading indicator during HTTP requests
 */
@Injectable()
export class LoadingInterceptor implements HttpInterceptor {
  private totalRequests = 0;

  constructor(private loadingService: LoadingService) {}

  intercept(request: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Don't show loader for certain requests
    if (this.shouldSkipLoading(request)) {
      return next.handle(request);
    }

    this.totalRequests++;
    this.loadingService.start();

    return next.handle(request).pipe(
      tap(event => {
        // If the response is successful, check if it's the final response
        if (event instanceof HttpResponse) {
          if (this.totalRequests <= 1) {
            this.loadingService.stop();
          }
        }
      }),
      finalize(() => {
        this.totalRequests--;
        
        if (this.totalRequests === 0) {
          this.loadingService.stop();
        }
      })
    );
  }

  /**
   * Check if loading indicator should be skipped for this request
   */
  private shouldSkipLoading(request: HttpRequest<any>): boolean {
    // Skip loading indicator for polling requests
    const isPollRequest = request.headers.has('x-skip-loading') 
      || request.url.includes('/status') 
      || request.url.includes('/health');
    
    // Skip loading indicator for background requests
    const isBackgroundRequest = request.headers.has('x-background-request');
    
    return isPollRequest || isBackgroundRequest;
  }
}
