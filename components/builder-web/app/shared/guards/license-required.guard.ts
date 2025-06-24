import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    const license = this.store.getState().users.current.license;
    if (!license || !license.licenseKey || this.expiredLicense(license.expirationDate)) {
      this.router.navigate(['/profile']);
      return false;
    }
    return true;
  }

  expiredLicense(expirationDate): boolean {
    const now = new Date();
    const exp = new Date(expirationDate);
    return exp < now;
  }

}
