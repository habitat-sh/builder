// Copyright (c) 2018-2025 Progress Software Corporation and/or its subsidiaries, affiliates or applicable contributors. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { OriginPageComponent } from '../origin-page/origin-page.component';
import { OriginKeysTabComponent } from './origin-keys-tab/origin-keys-tab.component';
import { OriginMembersTabComponent } from './origin-members-tab/origin-members-tab.component';
import { OriginPackagesTabComponent } from './origin-packages-tab/origin-packages-tab.component';
import { OriginSettingsTabComponent } from './origin-settings-tab/origin-settings-tab.component';
import { OriginIntegrationsTabComponent } from './origin-integrations-tab/origin-integrations-tab.component';
import { OriginJobsTabComponent } from './origin-jobs-tab/origin-jobs-tab.component';
import { OriginJobDetailComponent } from './origin-job-detail/origin-job-detail.component';
import { BuilderEnabledGuard } from '../../shared/guards/builder-enabled.guard';
import { VisibilityEnabledGuard } from '../../shared/guards/visibility-enabled.guard';
import { OriginMemberGuard } from '../../shared/guards/origin-member.guard';
import { SignedInGuard } from '../../shared/guards/signed-in.guard';
import { LicenseRequiredGuard } from '../../shared/guards/license-required.guard';

const routes: Routes = [
  {
    path: 'origins/:origin',
    component: OriginPageComponent,
    canActivate: [SignedInGuard, LicenseRequiredGuard],
    children: [
      {
        path: '',
        redirectTo: 'packages',
        pathMatch: 'full',
        canActivate: [SignedInGuard, LicenseRequiredGuard]
      },
      {
        path: 'packages',
        component: OriginPackagesTabComponent,
        canActivate: [SignedInGuard, LicenseRequiredGuard]
      },
      {
        path: 'keys',
        component: OriginKeysTabComponent,
        canActivate: [SignedInGuard, LicenseRequiredGuard]
      },
      {
        path: 'members',
        component: OriginMembersTabComponent,
        canActivate: [SignedInGuard, OriginMemberGuard, LicenseRequiredGuard],
      },
      {
        path: 'settings',
        component: OriginSettingsTabComponent,
        canActivate: [VisibilityEnabledGuard, SignedInGuard, OriginMemberGuard, LicenseRequiredGuard],
      },
      {
        path: 'integrations',
        component: OriginIntegrationsTabComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard, LicenseRequiredGuard]
      },
      {
        path: 'jobs',
        component: OriginJobsTabComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard, LicenseRequiredGuard]
      },
      {
        path: 'jobs/:id',
        component: OriginJobDetailComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard, LicenseRequiredGuard]
      },
      {
        path: '**',
        redirectTo: 'packages',
        canActivate: [SignedInGuard, LicenseRequiredGuard]
      }
    ]
  }
];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule]
})
export class OriginPageRoutingModule { }
