import { inject } from '@angular/core';
import { ActivatedRouteSnapshot, CanActivateFn, Router, RouterStateSnapshot, UrlTree } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';
import { OriginService } from '../services/origin.service';
import { firstValueFrom } from 'rxjs';

/**
 * Origin member guard for routes that require origin membership
 */
export const originMemberGuard: CanActivateFn = 
  async (route: ActivatedRouteSnapshot, state: RouterStateSnapshot): Promise<boolean | UrlTree> => {
    const authService = inject(AuthService);
    const originService = inject(OriginService);
    const router = inject(Router);
    
    const originName = route.params['origin'];
    
    if (!originName) {
      return false;
    }
    
    if (!authService.isAuthenticated()) {
      // Store the attempted URL for redirecting after login
      authService.setRedirectUrl(state.url);
      return router.createUrlTree(['/sign-in']);
    }
    
    try {
      const members = await firstValueFrom(originService.getOriginMembers(originName));
      const isOriginMember = members.some(member => 
        member.userId === authService.currentUser()?.id);
      
      if (isOriginMember) {
        return true;
      }
      
      // Redirect to origin page if not a member
      return router.createUrlTree(['/origins', originName]);
    } catch (error) {
      console.error('Error checking origin membership', error);
      return router.createUrlTree(['/']);
    }
  };
