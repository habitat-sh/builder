import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {
  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    const licenseValid = this.store.getState().users.current.licenseValid;
    if (!licenseValid) {
      this.router.navigate(['/profile']);
      return false;
    }
    return true;
  }
}
