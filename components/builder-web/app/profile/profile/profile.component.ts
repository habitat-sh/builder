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

import { Component, OnInit, OnDestroy } from '@angular/core';
import { MatDialog } from '@angular/material';
import { Title } from '@angular/platform-browser';
import { AppStore } from '../../app.store';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { clearAccessTokens, clearNewAccessToken, deleteAccessToken, fetchProfile, fetchAccessTokens, generateAccessToken, saveProfile, fetchLicenseKey, saveLicenseKey, signOut } from '../../actions/index';
import { ValidLicenseConfirmDialog } from '../../shared/dialog/valid-license-confirm/valid-license-confirm.dialog';
import { Location } from '@angular/common';
import { Router } from '@angular/router';

@Component({
  template: require('./profile.component.html')
})
export class ProfileComponent implements OnInit, OnDestroy {
  licenseValidationMessage: string;
  private dialogRef: any;
  private allSubscriptions: any[] = [];

  constructor(
    private confirmDialog: MatDialog,
    private store: AppStore,
    private title: Title,
    private location: Location,
    private router: Router
  ) {
    this.title.setTitle(`My Profile | ${store.getState().app.name}`);
  }

  ngOnInit() {
    this.fetch();
    this.store.dispatch(fetchLicenseKey(this.token));

    // Only run license dialog logic in SaaS mode
    if (this.config.is_saas) {
      this.allSubscriptions.push(this.store.observe('users.current.license').subscribe(({licenseKey, expirationDate, saveLicenseKeyErrorMessage}) => {
        // Only run dialog logic if on /profile
        if (!this.router.url.startsWith('/profile')) {
          if (this.dialogRef) {
            this.dialogRef.close();
            this.dialogRef = null;
          }
          return;
        }

        let errorMsg = '';
        let showDialog = false;

        // --- License dialog/message logic ---
        // Determine if the license is expired
        const isExpired = expirationDate ? new Date(expirationDate) < new Date() : false;
        // Always clear error message initially
        this.licenseValidationMessage = '';
        // License dialog and validation message logic
        // 1. If license is expired, show dialog with expired message
        // 2. If license is missing, show dialog (with backend error if present)
        // 3. If license is valid, close dialog and show valid message
        if (expirationDate && isExpired) {
          // Expired license: show dialog with specific error
          errorMsg = 'Your license has expired. Re-enter a new license key to download packages';
          this.licenseValidationMessage = errorMsg;
          showDialog = true;
        } else if (!licenseKey && this.token) {
          // No license: show dialog, show backend error if present
          errorMsg = saveLicenseKeyErrorMessage || '';
          this.licenseValidationMessage = '';
          showDialog = true;
        } else if (licenseKey && (!expirationDate || !isExpired)) {
          // Valid license: close dialog and show valid message
          this.licenseValidationMessage = expirationDate ? `License valid till ${expirationDate}` : '';
          if (this.dialogRef) {
            this.dialogRef.close();
            this.dialogRef = null;
          }
        }
        // If dialog should be shown, open or update it
        if (showDialog) {
          if (!this.dialogRef) {
            this.openAndSetupDialog(errorMsg, 'add');
          } else {
            this.dialogRef.componentInstance.setErrorMessage(errorMsg);
          }
        }
      }));
    }
  }

  // Opens and sets up the license dialog for entering or updating a license key.
  // - Only opens if on /profile route.
  // - Sets error message if provided.
  // - Handles proceed action: validates input and saves license key.
  // - Ensures dialog cannot be closed unless a valid license is entered (in 'add' mode).
  // - If dialog is closed without a valid license, reopens dialog if still on /profile and license is missing.
  openAndSetupDialog(saveLicenseKeyErrorMessage: string = '', mode: string = 'add') {
    // Only open dialog if still on /profile
    if (!this.router.url.startsWith('/profile')) {
      return;
    }
    this.openConfirmDialog({mode: mode});
    this.dialogRef.componentInstance.setErrorMessage(saveLicenseKeyErrorMessage || '');
    this.dialogRef.componentInstance.proceed = () => {
      const licenseKey = this.dialogRef.componentInstance.licenseKey?.trim();
      if (!licenseKey) {
        // Do not proceed if license key is empty
        return;
      }
      this.saveLicenseKeyToBackend({ licenseKey });
    };
    this.dialogRef.afterClosed().subscribe((data) => {
      this.dialogRef = null;
      // If dialog closed without data and license is still missing, re-open dialog
      if (!data) {
        if (!this.licenseKey && this.router.url.startsWith('/profile')) {
          this.openAndSetupDialog('', 'add');
        }
        return;
      }
      const { licenseKey } = data || {};
      if (licenseKey) {
        this.saveLicenseKeyToBackend(licenseKey);
      }
    });
  }

  ngOnDestroy() {
    // Guard against missing unsubscribe method
    this.allSubscriptions.forEach(sub => sub && typeof sub.unsubscribe === 'function' && sub.unsubscribe());
    this.dialogRef = null;
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

  saveLicenseKeyToBackend({licenseKey}) {
    this.store.dispatch(saveLicenseKey(licenseKey, this.token, this.profile.id));
  }

  updateKey() {
    this.openAndSetupDialog('', 'update');
  }

  openConfirmDialog({mode = 'add'}) {
    this.dialogRef = this.confirmDialog.open(ValidLicenseConfirmDialog, {
      width: '480px',
      disableClose: mode === 'add' ? true : false, // Prevent closing by clicking outside
      data: {
        heading: 'A Valid key is required for viewing and downloading the packages on the builder.',
        body: ``,
        action: `Proceed`,
        mode: mode
      }
    });
  }

  get licenseKey() {
    return this.store.getState().users.current.license.licenseKey;
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
