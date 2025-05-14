import { Routes } from '@angular/router';
import { OAuthCallbackComponent } from './oauth-callback/oauth-callback.component';
import { OAuthTokenComponent } from './oauth-token/oauth-token.component';
import { MockOAuthCallbackComponent } from './mock-oauth-callback/mock-oauth-callback.component';
import { DevAuthComponent } from './development/dev-auth.component';
import { environment } from '../../../environments/environment';

export const AUTH_ROUTES: Routes = [
  {
    path: 'callback',
    component: OAuthCallbackComponent
  },
  {
    path: 'oauth-token',
    component: OAuthTokenComponent
  },
  {
    path: 'mock-callback',
    component: MockOAuthCallbackComponent
  },
  {
    path: 'dev',
    component: DevAuthComponent,
    // Only available in non-production environments
    canActivate: [
      () => !environment.production
    ]
  },
  {
    path: 'sign-in',
    redirectTo: '/sign-in',
    pathMatch: 'full'
  },
  {
    path: '',
    redirectTo: '/sign-in',
    pathMatch: 'full'
  }
];
