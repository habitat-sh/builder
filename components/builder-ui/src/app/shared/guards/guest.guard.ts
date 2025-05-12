import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';

/**
 * Guest guard for routes that should only be accessible to non-authenticated users
 * (like login page to prevent logged-in users from accessing it)
 */
export const guestGuard: CanActivateFn = 
  (): boolean | UrlTree => {
    const authService = inject(AuthService);
    const router = inject(Router);
    
    if (!authService.isAuthenticated()) {
      return true;
    }
    
    // If user is already authenticated, redirect to home
    return router.createUrlTree(['/']);
  };
