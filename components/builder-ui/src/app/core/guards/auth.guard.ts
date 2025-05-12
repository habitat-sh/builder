import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { AuthService } from '../services/auth.service';

/**
 * Guard that prevents access to routes unless the user is authenticated
 * Also handles token refresh if the token is about to expire
 */
export const authGuard: CanActivateFn = (route, state) => {
  const authService = inject(AuthService);
  const router = inject(Router);
  
  // Check if user is authenticated
  if (authService.isAuthenticated()) {
    // Check if token needs refresh
    if (authService.isTokenExpired()) {
      console.log('AuthGuard: Token near expiration, refreshing...');
      
      // Return a Promise that resolves when refresh completes
      return new Promise<boolean | UrlTree>(resolve => {
        authService.refreshToken().subscribe({
          next: (refreshed) => {
            if (refreshed) {
              console.log('AuthGuard: Token refreshed successfully');
              resolve(true);
            } else {
              console.log('AuthGuard: Token refresh failed, redirecting to login');
              authService.setRedirectUrl(state.url);
              resolve(router.createUrlTree(['/sign-in']));
            }
          },
          error: () => {
            console.error('AuthGuard: Token refresh error, redirecting to login');
            authService.setRedirectUrl(state.url);
            resolve(router.createUrlTree(['/sign-in']));
          }
        });
      });
    }
    
    // Token is valid and not near expiration
    return true;
  }
  
  // Store the attempted URL for redirecting after login
  authService.setRedirectUrl(state.url);
  console.log('AuthGuard: User not authenticated, redirecting to login');
  
  // Redirect to sign-in page
  return router.createUrlTree(['/sign-in']);
};

/**
 * Guard that prevents access to routes if the user is already authenticated
 * (useful for login/register routes)
 */
export const noAuthGuard: CanActivateFn = () => {
  const authService = inject(AuthService);
  const router = inject(Router);
  
  if (!authService.isAuthenticated()) {
    return true;
  }
  
  // Redirect to home if already logged in
  return router.createUrlTree(['/home']);
};

/**
 * Guard that checks if the user has specific permissions to access a route
 */
export function permissionGuard(requiredPermission: string): CanActivateFn {
  return () => {
    const authService = inject(AuthService);
    const router = inject(Router);
    
    if (authService.hasPermission(requiredPermission)) {
      return true;
    }
    
    // Redirect to unauthorized page
    return router.createUrlTree(['/unauthorized']);
  };
}

/**
 * Guard that checks if the user has a specific role to access a route
 */
export function roleGuard(requiredRoles: string | string[]): CanActivateFn {
  return () => {
    const authService = inject(AuthService);
    const router = inject(Router);
    
    if (authService.hasRole(requiredRoles)) {
      return true;
    }
    
    // Redirect to unauthorized page
    return router.createUrlTree(['/unauthorized']);
  };
}
