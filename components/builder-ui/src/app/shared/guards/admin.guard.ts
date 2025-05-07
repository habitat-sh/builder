import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';

/**
 * Admin guard for routes that require administrator privileges
 */
export const adminGuard: CanActivateFn = 
  (): boolean | UrlTree => {
    const authService = inject(AuthService);
    const router = inject(Router);
    
    if (!authService.isAuthenticated()) {
      // Store the attempted URL for redirecting after login
      authService.setRedirectUrl(router.url);
      return router.createUrlTree(['/sign-in']);
    }
    
    if (authService.hasRole('admin')) {
      return true;
    }
    
    // If user is not an admin, redirect to dashboard
    return router.createUrlTree(['/']);
  };
