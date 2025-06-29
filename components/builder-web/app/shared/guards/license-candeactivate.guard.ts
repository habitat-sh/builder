import { Injectable } from '@angular/core';
import { CanDeactivate } from '@angular/router';
import { ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { ProfileComponent } from '../../profile/profile/profile.component';
import { AppStore } from '../../app.store';
import config from '../../config';
import { routeChange, signOut } from '../../actions';

@Injectable({ providedIn: 'root' })
export class LicenseCanDeactivateGuard implements CanDeactivate<ProfileComponent> {
  constructor(private store: AppStore) {}

  canDeactivate(
    component: ProfileComponent,
    _currentRoute: ActivatedRouteSnapshot,
    _currentState: RouterStateSnapshot,
    nextState?: RouterStateSnapshot
  ): boolean {
    if (!config.is_saas) return true;
    // Allow navigation to /sign-in
    if (nextState && nextState.url && nextState.url.startsWith('/sign-in')) {
      // If the user is navigating to /sign-in, allow it regardless of license validity
      return true;
    }

    const islicenseValid = this.store.getState().users.current.license.isValid;

    // Allow navigation to /sign-in
    if (nextState && nextState.url && nextState.url.startsWith('/sign-in')) {
      return true;
    }

    if (!islicenseValid) {
      // Block navigation away from /profile if license is not valid
      // Optionally show a message here
      this.store.dispatch(signOut(true));
      this.store.dispatch(routeChange(['/sign-in']));
      return false;
    }

    // Allow navigation if license is valid
    return true;
  }
}
