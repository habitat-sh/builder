import { APP_INITIALIZER } from '@angular/core';
import { AuthService } from '../services/auth.service';

/**
 * Factory function to initialize authentication state before app loads
 * This ensures that any token refresh or auth validation happens
 * before app components are rendered
 */
export function initializeAuth(authService: AuthService) {
  return () => {
    console.log('AuthInitializer: Checking authentication state on app startup');
    
    // Record the authentication initialization time
    const startTime = Date.now();
    
    // Return a promise that resolves when auth check is complete
    return new Promise<void>((resolve) => {
      // Check if token needs refresh
      if (authService.isAuthenticated() && authService.isTokenExpired()) {
        console.log('AuthInitializer: Token requires refresh, attempting refresh');
        authService.refreshToken().subscribe({
          next: (success) => {
            console.log('AuthInitializer: Token refresh completed', 
              success ? 'successfully' : 'with failure');
            const duration = Date.now() - startTime;
            console.log(`AuthInitializer: Auth initialization completed in ${duration}ms`);
            resolve();
          },
          error: () => {
            console.log('AuthInitializer: Token refresh failed, proceeding with app initialization');
            const duration = Date.now() - startTime;
            console.log(`AuthInitializer: Auth initialization completed in ${duration}ms`);
            resolve();
          }
        });
      } else {
        console.log('AuthInitializer: No token refresh needed');
        const duration = Date.now() - startTime;
        console.log(`AuthInitializer: Auth initialization completed in ${duration}ms`);
        resolve();
      }
    });
  };
}

/**
 * Auth initializer provider for APP_INITIALIZER
 */
export const authInitializerProvider = {
  provide: APP_INITIALIZER,
  useFactory: initializeAuth,
  deps: [AuthService],
  multi: true
};
