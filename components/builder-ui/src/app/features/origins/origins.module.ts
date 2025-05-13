import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { Routes } from '@angular/router';
import { SharedModule } from '../../shared/shared.module';

import { OriginListComponent } from './pages/origin-list/origin-list.component';
import { OriginDetailComponent } from './pages/origin-detail/origin-detail.component';
import { OriginOverviewComponent } from './pages/origin-detail/origin-overview.component';
import { OriginSettingsComponent } from './pages/origin-detail/origin-settings.component';
import { OriginMembersComponent } from './pages/origin-detail/origin-members.component';
import { OriginKeysComponent } from './pages/origin-detail/origin-keys.component';
import { OriginIntegrationsComponent } from './pages/origin-detail/origin-integrations.component';

const routes: Routes = [
  { path: '', component: OriginListComponent },
  { 
    path: ':origin', 
    component: OriginDetailComponent,
    children: [
      { path: '', component: OriginOverviewComponent },
      { path: 'settings', component: OriginSettingsComponent },
      { path: 'members', component: OriginMembersComponent },
      { path: 'keys', component: OriginKeysComponent },
      { path: 'integrations', component: OriginIntegrationsComponent },
    ]
  }
];

@NgModule({
  imports: [
    CommonModule,
    RouterModule.forChild(routes),
    SharedModule,
    OriginListComponent,
    OriginDetailComponent,
    OriginOverviewComponent,
    OriginSettingsComponent,
    OriginMembersComponent,
    OriginKeysComponent,
    OriginIntegrationsComponent,
  ],
  exports: [
    RouterModule
  ]
})
export class OriginsModule { }
