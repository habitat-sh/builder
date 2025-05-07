import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';

/**
 * Auth guard for protected routes that require authentication
 */
export const authGuard: CanActivateFn = 
  (): boolean | UrlTree => {
    const authService = inject(AuthService);
    const router = inject(Router);
    
    if (authService.isAuthenticated()) {
      return true;
    }
    
    // Store the attempted URL for redirecting after login
    authService.setRedirectUrl(router.url);
    
    // Navigate to the login page with extras
    return router.createUrlTree(['/sign-in']);
  };
