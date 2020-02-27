// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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
import { PackageComponent } from './package/package.component';
import { PackageJobComponent } from './package-job/package-job.component';
import { PackageJobsComponent } from './package-jobs/package-jobs.component';
import { PackageLatestComponent } from './package-latest/package-latest.component';
import { PackageSettingsComponent } from './package-settings/package-settings.component';
import { PackageReleaseComponent } from './package-release/package-release.component';
import { PackageReleaseSettingsComponent } from './package-release-settings/package-release-settings.component';
import { PackageVersionsComponent } from './package-versions/package-versions.component';
import { BuilderEnabledGuard } from '../shared/guards/builder-enabled.guard';
import { OriginMemberGuard } from '../shared/guards/origin-member.guard';
import { SignedInGuard } from '../shared/guards/signed-in.guard';

const routes: Routes = [
  {
    path: 'pkgs/:origin/:name',
    component: PackageComponent,
    children: [
      {
        path: '',
        component: PackageVersionsComponent,
      },
      {
        path: 'latest',
        component: PackageLatestComponent
      },
      {
        path: 'latest/:target',
        component: PackageLatestComponent
      },
      {
        path: 'jobs',
        component: PackageJobsComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard]
      },
      {
        path: 'jobs/:id',
        component: PackageJobComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard]
      },
      {
        path: 'settings',
        component: PackageSettingsComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard]
      },
      {
        path: 'settings/:target',
        component: PackageSettingsComponent,
        canActivate: [BuilderEnabledGuard, SignedInGuard, OriginMemberGuard]
      },
      {
        path: ':version',
        component: PackageVersionsComponent
      },
      {
        path: ':version/:release',
        component: PackageReleaseComponent
      },
      {
        path: ':version/:release/settings',
        component: PackageReleaseSettingsComponent
      }
    ]
  }
];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule]
})
export class PackageRoutingModule { }
