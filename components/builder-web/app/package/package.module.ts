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
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { RouterModule } from '@angular/router';
import { MatTabsModule, MatButtonModule, MatRadioModule } from '@angular/material';
import { JobDetailComponent } from './job-detail/job-detail.component';
import { JobListComponent } from './job-list/job-list.component';
import { PackageJobComponent } from './package-job/package-job.component';
import { PackageComponent } from './package/package.component';
import { PackageJobsComponent } from './package-jobs/package-jobs.component';
import { PackageDetailComponent } from './package-detail/package-detail.component';
import { PackageLatestComponent } from './package-latest/package-latest.component';
import { PackagePromoteComponent } from './package-promote/package-promote.component';
import { PackageSettingsComponent } from './package-settings/package-settings.component';
import { PackageReleaseComponent } from './package-release/package-release.component';
import { PackageSidebarComponent } from './package-sidebar/package-sidebar.component';
import { PackageVersionsComponent } from './package-versions/package-versions.component';
import { PackageCreateDialog } from './package-create-dialog/package-create.dialog';
import { SharedModule } from '../shared/shared.module';
import { PackageRoutingModule } from './package-routing.module';

@NgModule({
  imports: [
    CommonModule,
    FormsModule,
    PackageRoutingModule,
    ReactiveFormsModule,
    RouterModule,
    MatTabsModule,
    MatButtonModule,
    MatRadioModule,
    SharedModule,
    FormsModule,
    ReactiveFormsModule
  ],
  declarations: [
    JobDetailComponent,
    JobListComponent,
    PackageComponent,
    PackageCreateDialog,
    PackageJobComponent,
    PackageJobsComponent,
    PackageLatestComponent,
    PackageDetailComponent,
    PackagePromoteComponent,
    PackageReleaseComponent,
    PackageSidebarComponent,
    PackageSettingsComponent,
    PackageVersionsComponent
  ],
  entryComponents: [
    PackageCreateDialog
  ],
  exports: []
})
export class PackageModule { }
