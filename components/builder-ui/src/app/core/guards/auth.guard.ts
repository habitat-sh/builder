import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { AuthService } from '../services/auth.service';

/**
 * Guard that prevents access to routes unless the user is authenticated
 */
export const authGuard: CanActivateFn = (route, state) => {
  const authService = inject(AuthService);
  const router = inject(Router);
  
  if (authService.isAuthenticated()) {
    return true;
  }
  
  // Redirect to login page with return url
  const returnUrl = state.url;
  return router.createUrlTree(['/auth/login'], { queryParams: { returnUrl } });
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
  
  // Redirect to dashboard if already logged in
  return router.createUrlTree(['/dashboard']);
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
