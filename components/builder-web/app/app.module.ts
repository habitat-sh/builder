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
import { LocationStrategy, HashLocationStrategy } from '@angular/common';
import { MatButtonModule } from '@angular/material/button';
import { MatDialogModule } from '@angular/material/dialog';
import { MatIconModule } from '@angular/material/icon';
import { MatRadioModule } from '@angular/material/radio';
import { MatTabsModule } from '@angular/material/tabs';
import { MAT_FORM_FIELD_DEFAULT_OPTIONS } from '@angular/material/form-field';
import { BrowserModule } from '@angular/platform-browser';
import { BrowserAnimationsModule } from '@angular/platform-browser/animations';
import { HttpClientModule } from '@angular/common/http';
import { RouterOutlet } from '@angular/router';
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
    MatDialogModule,
    BrowserModule,
    BrowserAnimationsModule,
    FormsModule,
    HttpClientModule,
    MatButtonModule,
    OriginModule,
    PackageModule,
    ProfileModule,
    ReactiveFormsModule,
    SearchModule,
    EventsModule,
    SharedModule,
    RouterOutlet,
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
    { provide: MAT_FORM_FIELD_DEFAULT_OPTIONS, useValue: { floatLabel: 'always' } },
    AppStore
  ],
  bootstrap: [AppComponent]
})
export class AppModule {}

