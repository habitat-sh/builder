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
import { Router, NavigationEnd } from '@angular/router';
import config from '../../config';

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
    private router: Router
  ) {
    this.title.setTitle(`My Profile | ${store.getState().app.name}`);
  }

  ngOnInit() {
    this.fetch();

    if (this.config.is_saas) {
      this.store.dispatch(fetchLicenseKey(this.token));
      // Listen for license state changes
      this.allSubscriptions.push(
        this.store.observe('users.current.license').subscribe((license) => {
          if (this.router.url.startsWith('/profile')) {
            // Always check and show dialog if license is invalid or error
            this.checkAndShowLicenseDialog();
          } else if (this.dialogRef) {
            // If user leaves /profile, close dialog
            this.dialogRef.close();
            this.dialogRef = null;
          }
        })
      );
    }
  }

  // Extract dialog logic to a method
  checkAndShowLicenseDialog() {
    const { expirationDate, saveLicenseKeyErrorMessage, licenseFetchInProgress, isValid } = this.store.getState().users.current.license;

    let showDialog = false;
    let errorMsg = saveLicenseKeyErrorMessage || '';
    let mode = 'add';

    if (licenseFetchInProgress) {
      return;
    }

    const isExpired = expirationDate && new Date(expirationDate) < new Date();

    if (!isValid && this.token) {
      if (isExpired && !saveLicenseKeyErrorMessage) {
        // First time expired: show the expired message
        errorMsg = 'Your license has expired. Re-enter a new license key to download packages.';
      } else {
        // Later, show backend error message (if any)
        errorMsg = saveLicenseKeyErrorMessage || '';
      }
      if (isExpired) {
        this.licenseValidationMessage = 'Your license has expired. Re-enter a new license key to download packages.';
      }
      showDialog = true;
      mode = 'add';
    } else {
      this.licenseValidationMessage = `License valid till ${expirationDate}`;
      showDialog = false;
      if (this.dialogRef) {
        this.dialogRef.close();
        this.dialogRef = null;
      }
      return;
    }

    // Always close any open dialog before opening a new one if showDialog is true
    if (showDialog) {
      // Only open a new dialog if none is currently open
      if (!this.dialogRef) {
        this.openAndSetupDialog(errorMsg, mode);
      } else {
        // If dialog is open with the same mode, just update the error message if it changed
        const currentMode = this.dialogRef?.componentInstance?.mode;
        if (currentMode === mode) {
          const currentMsg = this.dialogRef?.componentInstance?.errorMessage;
          if (currentMsg !== errorMsg) {
            this.dialogRef.componentInstance.setErrorMessage(errorMsg);
          }
        }
        // else: dialog is open with a different mode, do nothing
      }
    }
  }

  // Opens and sets up the license dialog for entering or updating a license key.
  // - Only opens if on /profile route.
  // - Sets error message if provided.
  // - Handles proceed action: validates input and saves license key.
  // - Ensures dialog cannot be closed unless a valid license is entered (in 'add' mode).
  // - If dialog is closed without a valid license, reopens dialog if still on /profile and license is missing.
  openAndSetupDialog(error: string = '', mode: string = 'add') {
    if (!this.router.url.startsWith('/profile')) return;
    if (this.dialogRef) return; // Don't open multiple dialogs
    const dialogRef = this.confirmDialog.open(ValidLicenseConfirmDialog, {
      width: '480px',
      disableClose: mode === 'add' ? true : false, // Prevent closing by clicking outside
      data: {
        heading: 'A Valid key is required for viewing and downloading the packages on the builder.',
        body: ``,
        action: `Proceed`,
        mode: mode
      }
    });
    this.dialogRef = dialogRef;
    dialogRef.componentInstance.setErrorMessage(error || '');
    dialogRef.componentInstance.proceed = () => {
      const licenseKey = dialogRef.componentInstance.licenseKey?.trim();
      if (!licenseKey) {
        dialogRef.componentInstance.setErrorMessage('Please enter a license key.');
        return;
      }
      this.saveLicenseKeyToBackend({ licenseKey });
    };

    // Handle dialog close event
    // If dialog is closed without a valid license, reopen if still on /profile
    // This ensures that if the user navigates away and comes back, they are prompted again
    dialogRef.afterClosed().subscribe(() => {
      if (this.dialogRef === dialogRef) {
        this.dialogRef = null;
      }
      // If dialog is closed without a valid license, reopen if still on /profile
      if (this.router.url.startsWith('/profile')) {
        this.checkAndShowLicenseDialog();
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
