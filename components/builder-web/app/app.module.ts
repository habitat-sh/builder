// Copyright (c) 2016-2021 Chef Software Inc. and/or applicable contributors
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
import { RouterModule } from '@angular/router';
import { LocationStrategy, HashLocationStrategy } from '@angular/common';
import { MatButtonModule, MatIconModule, MatRadioModule, MatTabsModule, MAT_LABEL_GLOBAL_OPTIONS } from '@angular/material';
import { BrowserModule } from '@angular/platform-browser';
import { HttpClientModule } from '@angular/common/http';
import { routing } from './routes';
import { AppStore } from './app.store';
import { AppComponent } from './app.component';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { NotificationsComponent } from './notifications/notifications.component';
import { SideNavComponent } from './side-nav/side-nav.component';
import { SignInPageComponent } from './sign-in-page/sign-in-page.component';
import { StatuspageIndicatorComponent } from './statuspage/statuspage-indicator.component';
import { UserNavComponent } from './user-nav/user-nav.component';
import { OriginModule } from './origin/origin.module';
import { PackageModule } from './package/package.module';
import { ProfileModule } from './profile/profile.module';
import { SearchModule } from './search/search.module';
import { EventsModule } from './events/events.module';
import { SharedModule } from './shared/shared.module';

@NgModule({
  imports: [
    MatIconModule,
    MatRadioModule,
    MatTabsModule,
    BrowserModule,
    FormsModule,
    HttpClientModule,
    MatButtonModule,
    OriginModule,
    PackageModule,
    ProfileModule,
    ReactiveFormsModule,
    RouterModule,
    SearchModule,
    EventsModule,
    SharedModule,
    routing
  ],
  declarations: [
    AppComponent,
    NotificationsComponent,
    SideNavComponent,
    SignInPageComponent,
    StatuspageIndicatorComponent,
    UserNavComponent
  ],
  providers: [
    { provide: LocationStrategy, useClass: HashLocationStrategy, },
    { provide: MAT_LABEL_GLOBAL_OPTIONS, useValue: { float: 'always' } },
    AppStore
  ],
  bootstrap: [AppComponent]
})
export class AppModule {}
