import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import config from '../../config';
import { requestRoute } from '../../actions';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    console.log('LicenseRequiredGuard: canActivate called');
    if (!config.is_saas) {
      return true;
    }

    const isSigningIn = this.store.getState().users.current.isSigningIn;
    if (isSigningIn) {
      return false; // Block navigation if signing in
    }

    // Block all navigation if not SaaS mode if user is not logged in
    const isLoggedIn = this.store.getState().session.token;
    console.log('LicenseRequiredGuard: isLoggedIn', isLoggedIn);
    if (!isLoggedIn) {
      console.log('LicenseRequiredGuard: Not logged in, redirecting to sign-in');
      this.router.navigate(['/sign-in'], { queryParams: { message: 'You need to sign-in to access Public Builder' } });
      return false;
    }

    const licenseValid = this.store.getState().users.current.license.isValid;
    console.log('LicenseRequiredGuard: licenseValid', licenseValid);

    if (licenseValid === null) {
      // License status not loaded yet, block navigation
      return false;
    }

    // If license is invalid, redirect to profile page
    if (!licenseValid) {
      this.store.dispatch(requestRoute(['/profile']));
      return false;
    }

    // If license is valid, allow navigation
    return true;
  }
}
