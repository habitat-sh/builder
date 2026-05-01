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

import { NgModule, ErrorHandler } from '@angular/core';
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

class VisibleErrorHandler implements ErrorHandler {
  handleError(error: any) {
    console.error('[VisibleErrorHandler]', error);
    const msg = (error && (error.message || JSON.stringify(error))) || String(error);
    const stack = (error && error.stack) || '';
    let banner = document.getElementById('_ng_error_banner');
    if (!banner) {
      banner = document.createElement('div');
      banner.id = '_ng_error_banner';
      banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99999;background:#c00;color:#fff;padding:12px;font:12px monospace;white-space:pre-wrap;max-height:40vh;overflow:auto;';
      document.body.appendChild(banner);
    }
    banner.textContent += '\n---\n' + msg + '\n' + stack;
  }
}

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
    { provide: ErrorHandler, useClass: VisibleErrorHandler },
    AppStore
  ],
  bootstrap: [AppComponent]
})
export class AppModule {}

