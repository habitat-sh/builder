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
    const license = this.store.getState().users.current.license;
    const isInvalid = !license || !license.licenseKey || this.expiredLicense(license.expirationDate);
    const isAlreadyOnProfile = this.router.url.startsWith('/profile');
    if (isInvalid && !isAlreadyOnProfile) {
      this.router.navigate(['/profile']);
      return false;
    }
    // If already on /profile, just block navigation but don't redirect again
    if (isInvalid && isAlreadyOnProfile) {
      return false;
    }
    // If license is valid, allow navigation
    return true;
  }

  expiredLicense(expirationDate): boolean {
    const now = new Date();
    const exp = new Date(expirationDate);
    return exp < now;
  }

}
