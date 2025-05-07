import { Routes } from '@angular/router';
import { LayoutComponent } from './core/layout/layout-components/layout.component';
import { NotFoundComponent } from './core/layout/layout-components/not-found.component';
import { DashboardComponent } from './features/dashboard/dashboard.component';

// Comment out the lazy loading for now
// import { PackagesComponent } from './features/packages/packages.component';
// import { OriginsComponent } from './features/origins/origins.component';
// import { BuildsComponent } from './features/builds/builds.component';
// import { ProfileComponent } from './features/profile/profile.component';
// import { SignInComponent } from './features/auth/sign-in/sign-in.component';
// import { OAuthTokenComponent } from './features/auth/oauth-token/oauth-token.component';

export const routes: Routes = [
  {
    path: '',
    component: LayoutComponent,
    children: [
      {
        path: '',
        component: DashboardComponent
      },
      {
        path: 'dashboard',
        component: DashboardComponent
      },
      // These paths will be implemented in future phases
      // { path: 'pkgs', component: PackagesComponent },
      // { path: 'origins', component: OriginsComponent },
      // { path: 'builds', component: BuildsComponent },
      // { path: 'profile', component: ProfileComponent },
    ]
  },
  // {
  //   path: 'sign-in',
  //   component: SignInComponent
  // },
  // {
  //   path: 'oauth-token',
  //   component: OAuthTokenComponent
  // },
  { path: '404', component: NotFoundComponent },
  { path: '**', redirectTo: '404' }
];
