import { Routes } from '@angular/router';
import { AppShellComponent } from './core/layout';
import { NotFoundComponent } from './core/layout/layout-components/not-found.component';
import { HomeComponent } from './features/dashboard/home.component';
import { ColorSwatchComponent } from './core/layout/color-swatch.component';
import { DebugAssetsComponent } from './debug/debug-assets.component';
import { PackageListComponent } from './features/packages/list/package-list.component';
import { PackageDetailComponent } from './features/packages/detail/package-detail.component';

// Guards
import { authGuard, roleGuard, permissionGuard } from './core/guards/auth.guard';
import { originMemberGuard } from './shared/guards/origin-member.guard';
import { guestGuard } from './shared/guards/guest.guard';
import { adminGuard } from './shared/guards/admin.guard';

export const routes: Routes = [
  // Public routes (with layout)
  {
    path: '',
    component: AppShellComponent,
    children: [
      {
        path: '',
        component: HomeComponent
      },
      {
        path: 'home',
        component: HomeComponent
      },
      {
        path: 'theme',
        component: ColorSwatchComponent
      },
      {
        path: 'debug/assets',
        component: DebugAssetsComponent
      },
      
      // Package routes
      {
        path: 'pkgs',
        component: PackageListComponent
      },
      {
        path: 'pkgs/:origin/:name',
        component: PackageDetailComponent
      },
      {
        path: 'pkgs/:origin/:name/:version',
        component: PackageDetailComponent
      },
      {
        path: 'pkgs/:origin/:name/:version/:release',
        component: PackageDetailComponent
      },
      
      // Protected routes - to be implemented in later phases
      {
        path: 'origins',
        canActivate: [authGuard],
        component: HomeComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'builds',
        canActivate: [authGuard],
        component: HomeComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'profile',
        canActivate: [authGuard],
        component: HomeComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'settings',
        canActivate: [authGuard],
        component: HomeComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'events',
        component: HomeComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'events/saas',
        component: HomeComponent // Placeholder - will be replaced with actual component
      }
    ]
  },
  
  // Auth routes (without layout)
  {
    path: 'auth',
    loadChildren: () => import('./features/auth/auth.module').then(m => m.AuthModule)
  },
  
  // Direct sign-in route (without auth path)
  {
    path: 'sign-in',
    loadComponent: () => import('./features/auth/sign-in/sign-in.component').then(c => c.SignInComponent)
  },
  
  // Admin routes - to be implemented in later phases
  {
    path: 'admin',
    canActivate: [authGuard, roleGuard('admin')],
    component: HomeComponent // Placeholder - will be replaced with actual component
  },
  
  // Error routes
  { path: '404', component: NotFoundComponent },
  { path: '**', redirectTo: '404' }
];
