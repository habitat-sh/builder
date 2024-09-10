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

import { Component, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { AppStore } from '../app.store';
import { setLayout, signOut } from '../actions/index';
import config from '../config';
import { EulaConfirmDialog } from '../shared/dialog/eula-confirm/eula-confirm.dialog';
import { MatDialog } from '@angular/material';

@Component({
  template: require('./sign-in-page.component.html')
})
export class SignInPageComponent implements OnDestroy {

  constructor(private store: AppStore, private title: Title, private confirmDialog: MatDialog) {
    store.dispatch(signOut(false));
    this.title.setTitle(`Sign In | ${store.getState().app.name}`);
    this.store.dispatch(setLayout('sign-in'));
  }

  get providerType() {
    return this.store.getState().oauth.provider.type;
  }

  get providerName() {
    return this.store.getState().oauth.provider.name;
  }

  get loginUrl() {
    const provider = this.store.getState().oauth.provider;

    const qs = Object.keys(provider.params)
      .map(k => `${k}=${encodeURIComponent(provider.params[k])}`)
      .join('&');

    return `${provider.authorizeUrl}?${qs}`;
  }

  get signupUrl() {
    return this.store.getState().oauth.provider.signupUrl;
  }

  get wwwUrl() {
    return config['www_url'];
  }

  ngOnDestroy() {
    this.store.dispatch(setLayout('default'));
  }

  showEulaPopup(URL, popupFor) {
    if (popupFor === 'signUp') {
      if (!localStorage.getItem('singUpShowEulaPopup') && !localStorage.getItem('singUpEulaAccept')) {
        this.confirmDialog
          .open(EulaConfirmDialog, {
            width: '530px',
            disableClose: true,
            data: {
              heading: 'End Users License Agreement',
              body: `I acknowledge and agree that use of Chef Habitat Builder is governed by and subject to the terms and conditions of the End User License Agreement for Chef`,
              action: 'Continue',
              signupUrl: URL
            }
          }).afterClosed()
          .subscribe((data) => {
            if (data) {
              localStorage.setItem('singUpEulaAccept', 'true');
              localStorage.setItem('singUpShowEulaPopup', 'false');
              window.open(URL);
            }
          });
      } else {
        window.open(URL);
      }
    } else {
      if (!localStorage.getItem('loginShowEulaPopup') && !localStorage.getItem('loginEulaAccept')) {
        this.confirmDialog
          .open(EulaConfirmDialog, {
            width: '530px',
            disableClose: true,
            data: {
              heading: 'End Users License Agreement',
              body: `I acknowledge and agree that use of Progress Chef Habitat Builder is governed by and subject to the terms and conditions of the End User License Agreement for Progress Chef`,
              action: 'Continue',
              signupUrl: URL
            }
          }).afterClosed()
          .subscribe((data) => {
            if (data) {
              localStorage.setItem('loginEulaAccept', 'true');
              localStorage.setItem('loginShowEulaPopup', 'false');
              window.open(this.loginUrl, '_self');
            }
          });
      } else {
        window.open(URL, '_self');
      }
    }
  }
}
