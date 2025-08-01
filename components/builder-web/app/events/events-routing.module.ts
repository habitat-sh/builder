// Copyright (c) 2021 Chef Software Inc. and/or applicable contributors
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

import { EventsComponent } from './events/events.component';
import { EventsSaaSComponent } from './events-saas/events-saas.component';
import { LicenseRequiredGuard } from '../shared/guards/license-required.guard';
import { SignedInGuard } from '../shared/guards/signed-in.guard';

const routes: Routes = [
  {
    path: 'events',
    component: EventsComponent,
    canActivate: [SignedInGuard, LicenseRequiredGuard]
  },
  {
    path: 'events/saas',
    component: EventsSaaSComponent,
    canActivate: [SignedInGuard, LicenseRequiredGuard]
  }
];

@NgModule({
  imports: [RouterModule.forChild(routes)],
  exports: [RouterModule]
})
export class EventsRoutingModule { }
