import { Routes } from '@angular/router';
import { OriginsListComponent } from './list/origins-list.component';
import { OriginDetailComponent } from './detail/origin-detail.component';
import { OriginOverviewComponent } from './detail/origin-overview.component';
import { OriginPlaceholderComponent } from './detail/origin-placeholder.component';
import { authGuard } from '../../core/guards/auth.guard'; 

// Ensure routes use the auth guard correctly
export const ORIGINS_ROUTES: Routes = [
  { 
    path: '', 
    component: OriginsListComponent 
  },
  {
    path: ':origin',
    component: OriginDetailComponent,
    children: [
      {
        path: '',
        component: OriginOverviewComponent
      },
      {
        path: 'settings',
        component: OriginPlaceholderComponent,
        data: { 
          title: 'Settings',
          message: 'Origin settings management will be implemented here.'
        }
      },
      {
        path: 'members',
        component: OriginPlaceholderComponent,
        data: { 
          title: 'Members',
          message: 'Origin members management will be implemented here.'
        }
      },
      {
        path: 'keys',
        component: OriginPlaceholderComponent,
        data: { 
          title: 'Keys',
          message: 'Origin keys management will be implemented here.'
        }
      },
      {
        path: 'integrations',
        component: OriginPlaceholderComponent,
        data: { 
          title: 'Integrations',
          message: 'Origin integrations management will be implemented here.'
        }
      }
    ]
  }
];
