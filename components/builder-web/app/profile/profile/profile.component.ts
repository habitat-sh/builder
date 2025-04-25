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

import { Component, OnInit, OnDestroy} from '@angular/core';
import { MatDialog } from '@angular/material';
import { Title } from '@angular/platform-browser';
import { AppStore } from '../../app.store';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { clearAccessTokens, clearNewAccessToken, deleteAccessToken, fetchProfile, fetchAccessTokens, generateAccessToken, saveProfile } from '../../actions/index';
import config from '../../config';

@Component({
  template: require('./profile.component.html')
})
export class ProfileComponent implements OnInit, OnDestroy {

  licenseKey = '';
  licenseValid = false;
  licenseValidationMessage = '';
  validatingLicenseKey = false;

  constructor(
    private confirmDialog: MatDialog,
    private store: AppStore,
    private title: Title
  ) {
    this.title.setTitle(`My Profile | ${store.getState().app.name}`);
  }

  ngOnInit() {
    this.fetch();
    this.fetchLicenseKey();
  }

  ngOnDestroy() {
    this.clearAccessTokens();
  }

  generateToken(regenerate = false) {
    if (regenerate) {
      this.confirmDialog
        .open(SimpleConfirmDialog, {
          width: '480px',
          data: {
            heading: 'Regenerate token',
            body: `Are you sure you want to regenerate your token? Doing so will invalidate and replace the current token.`,
            action: `I'm sure`
          }
        })
        .afterClosed()
        .subscribe((confirmed) => {
          if (confirmed) {
            this.store.dispatch(generateAccessToken(this.token));
          }
        });
    }
    else {
      this.store.dispatch(generateAccessToken(this.token));
    }
  }

  deleteToken(id) {
    this.confirmDialog
      .open(SimpleConfirmDialog, {
        width: '480px',
        data: {
          heading: 'Delete token',
          body: `Are you sure you want to delete this token? You will no longer be able to interact with Builder via the CLI.`,
          action: `I'm sure`
        }
      })
      .afterClosed()
      .subscribe((confirmed) => {
        if (confirmed) {
          this.store.dispatch(deleteAccessToken(id, this.token));
          this.clearAccessTokens();
        }
      });
  }

  save(form) {
    this.store.dispatch(saveProfile({ email: form.email }, this.token));
  }

  saveLicenseKeyToBackend() {
    this.validatingLicenseKey = true;
    this.licenseValidationMessage = '';
    this.licenseValid = false;

    const body = {
      account_id: this.profile.id,
      license_key: this.licenseKey
    };

    fetch('/v1/profile/license', {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${this.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(body)
    })
      .then(async res => {
        this.validatingLicenseKey = false;
        if (!res.ok) {
          const errorText = (await res.text()).trim();
          throw new Error(errorText || 'License validation failed.');
        }
        return res.json();
      })
      .then(data => {
        this.licenseValid = true;
        this.licenseValidationMessage = `License valid till ${data.expiration_date}`;
      })
      .catch(err => {
        this.licenseValid = false;
        this.licenseValidationMessage = err.message || 'License validation failed.';
      });
  }

  deleteLicenseKey() {
    fetch('/v1/profile/license', {
      method: 'DELETE',
      headers: {
        'Authorization': `Bearer ${this.token}`
      }
    })
      .then(res => {
        if (!res.ok) {
          throw new Error('Failed to delete license key.');
        }
        return res.text();
      })
      .then(() => {
        this.clearLicenseKey();
        console.log('License key deleted successfully.');
      })
      .catch(err => {
        this.licenseValidationMessage = err.message || 'License deletion failed.';
        console.error('Error deleting license key:', err);
      });
  }

  clearLicenseKey() {
    this.licenseKey = '';
    this.licenseValidationMessage = '';
    this.licenseValid = false;
  }

  private fetchLicenseKey() {
    fetch('/v1/profile/license', {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${this.token}`
      }
    })
      .then(res => res.json())
      .then(data => {
        if (data.license_key) {
          this.licenseKey = data.license_key;
          this.licenseValid = true;
          this.licenseValidationMessage = `License valid till ${data.expiration_date}`;
        }
      })
      .catch(err => {
        console.error('Error fetching license key from backend:', err);
      });
  }

  get accessToken() {
    return this.store.getState().users.current.accessTokens[0];
  }

  get config() {
    return config;
  }

  get newAccessToken() {
    return this.store.getState().users.current.newAccessToken;
  }

  get loadingAccessTokens() {
    return this.store.getState().users.current.ui.accessTokens.loading;
  }

  get generatingAccessToken() {
    return this.store.getState().users.current.ui.accessTokens.generating;
  }

  get deletingAccessToken() {
    return this.store.getState().users.current.ui.accessTokens.deleting;
  }

  get buttonLabel() {
    return (this.accessToken || this.newAccessToken) ? 'Regenerate' : 'Generate Token';
  }

  get processingLabel() {

    if (this.generatingAccessToken) {
      return 'Generating token';
    }

    if (this.deletingAccessToken) {
      return 'Deleting';
    }
  }

  get profile() {
    return this.store.getState().users.current.profile;
  }

  get providerType() {
    return this.store.getState().oauth.provider.type;
  }

  get providerName() {
    return this.store.getState().oauth.provider.name;
  }

  get token() {
    return this.store.getState().session.token;
  }

  private fetch() {
    this.store.dispatch(fetchProfile(this.token));
    this.store.dispatch(fetchAccessTokens(this.token));
  }

  private clearAccessTokens() {
    this.store.dispatch(clearAccessTokens());
    this.store.dispatch(clearNewAccessToken());
  }

  private handleUnauthorized(response: Response, reject: (reason?: any) => void): Response {
    if (response.status === 401) {
      reject(new Error('Unauthorized'));
    }
    return response;
  }

  private handleError(error: any, reject: (reason?: any) => void) {
    console.error('API error:', error);
    reject(error);
  }
}
