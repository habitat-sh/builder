import { Injectable } from '@angular/core';
import { HttpInterceptor, HttpRequest, HttpHandler, HttpEvent, HttpResponse } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { delay, mergeMap } from 'rxjs/operators';
import { environment } from '../../../environments/environment';

/**
 * Mock HTTP interceptor for authentication-related requests
 * This interceptor simulates API responses for auth endpoints during development
 */
@Injectable()
export class MockAuthInterceptor implements HttpInterceptor {
  
  // Mock user data
  private mockUser = {
    id: 'user-12345',
    name: 'Demo User',
    email: 'demo@example.com',
    avatar: 'https://avatars.githubusercontent.com/u/12345?v=4',
    role: 'contributor',
    permissions: ['read:packages', 'write:packages', 'read:origins', 'create:origins']
  };
  
  intercept(request: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    // Only intercept requests if mock mode is enabled
    if (!environment.useMocks) {
      return next.handle(request);
    }
    
    const url = request.url.toLowerCase();
    
    // Handle GitHub OAuth callback
    if (url.includes('/auth/github/callback')) {
      const mockToken = 'mock_github_token_' + Math.random().toString(36).substring(2);
      
      return this.delayResponse(new HttpResponse({
        status: 200,
        body: {
          token: mockToken,
          user: this.mockUser
        }
      }));
    }
    
    // Handle user data request
    if (url.includes('/auth/me') || url.includes('/users/me')) {
      // Check if authentication token is present
      const authToken = request.headers.get('Authorization');
      if (!authToken || !authToken.startsWith('Bearer ')) {
        return this.delayResponse(new HttpResponse({
          status: 401,
          body: { error: 'Unauthorized' }
        }));
      }
      
      return this.delayResponse(new HttpResponse({
        status: 200,
        body: this.mockUser
      }));
    }
    
    // Handle token refresh
    if (url.includes('/auth/refresh')) {
      const newToken = 'mock_refreshed_token_' + Math.random().toString(36).substring(2);
      
      return this.delayResponse(new HttpResponse({
        status: 200,
        body: { token: newToken }
      }));
    }
    
    // For all other requests, pass through
    return next.handle(request);
  }
  
  /**
   * Add a delay to simulate network latency
   */
  private delayResponse(response: HttpResponse<any>): Observable<HttpEvent<any>> {
    return of(response).pipe(delay(800 + Math.random() * 800));
  }
}
