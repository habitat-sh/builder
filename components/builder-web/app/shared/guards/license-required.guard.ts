import { Injectable, OnDestroy } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import { Subscription } from 'rxjs';

@Injectable()
export class LicenseRequiredGuard implements CanActivate, OnDestroy {
private licenseSubscription: Subscription;

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    this.licenseSubscription = this.store.observe('users.current.license.licenseKey').subscribe((licenseKey) => {
        if (!licenseKey) {
            this.router.navigate(['/profile']);
            return false;
        }
    });
    return true;
  }

  ngOnDestroy() {
    if (this.licenseSubscription) {
      this.licenseSubscription.unsubscribe();
    }
  }

}
