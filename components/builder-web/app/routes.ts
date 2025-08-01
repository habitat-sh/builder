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

import { Routes, RouterModule } from '@angular/router';
import { SignInPageComponent } from './sign-in-page/sign-in-page.component';
import { SignedInGuard } from './shared/guards/signed-in.guard';
import { LicenseRequiredGuard } from './shared/guards/license-required.guard';

export const routes: Routes = [
  {
    path: '',
    pathMatch: 'full',
    redirectTo: 'origins',
    canActivate: [SignedInGuard, LicenseRequiredGuard]
  },
  {
    path: 'sign-in',
    component: SignInPageComponent
  },
  {
    path: '*',
    redirectTo: '/pkgs/core',
    canActivate: [SignedInGuard, LicenseRequiredGuard]
  }
];

export const routing = RouterModule.forRoot(routes);
