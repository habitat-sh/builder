import { Routes } from '@angular/router';
import { AppShellComponent } from './core/layout';
import { NotFoundComponent } from './core/layout/layout-components/not-found.component';
import { DashboardComponent } from './features/dashboard/dashboard.component';
import { ColorSwatchComponent } from './core/layout/color-swatch.component';

// Guards
import { authGuard } from './shared/guards/auth.guard';
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
        component: DashboardComponent
      },
      {
        path: 'dashboard',
        component: DashboardComponent
      },
      {
        path: 'theme',
        component: ColorSwatchComponent
      },
      
      // Package routes - to be implemented in later phases
      {
        path: 'pkgs',
        component: DashboardComponent // Placeholder - will be replaced with actual component
      },
      
      // Protected routes - to be implemented in later phases
      {
        path: 'origins',
        canActivate: [authGuard],
        component: DashboardComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'builds',
        canActivate: [authGuard],
        component: DashboardComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'profile',
        canActivate: [authGuard],
        component: DashboardComponent // Placeholder - will be replaced with actual component
      },
      {
        path: 'settings',
        canActivate: [authGuard],
        component: DashboardComponent // Placeholder - will be replaced with actual component
      }
    ]
  },
  
  // Auth routes (without layout) - to be implemented in later phases
  {
    path: 'sign-in',
    canActivate: [guestGuard],
    component: DashboardComponent // Placeholder - will be replaced with actual component
  },
  
  // Admin routes - to be implemented in later phases
  {
    path: 'admin',
    canActivate: [authGuard, adminGuard],
    component: DashboardComponent // Placeholder - will be replaced with actual component
  },
  
  // Error routes
  { path: '404', component: NotFoundComponent },
  { path: '**', redirectTo: '404' }
];
