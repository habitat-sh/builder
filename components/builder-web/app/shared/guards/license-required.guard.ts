import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import config from '../../config';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    // Allow all navigation if not SaaS mode
    if (!config.is_saas) {
      return true;
    }
    const licenseValid = this.store.getState().users.current.license.isValid;

    // If license is invalid, redirect to profile page
    if (!licenseValid) {
      if (this.router.url !== '/profile') {
        this.router.navigate(['/profile']);
      }
      return false;
    }

    // If license is valid, allow navigation
    return true;
  }
}
