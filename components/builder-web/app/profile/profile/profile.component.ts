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
import { MatDialog, MatDialogRef } from '@angular/material';
import { Title } from '@angular/platform-browser';
import { AppStore } from '../../app.store';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { clearAccessTokens, clearNewAccessToken, deleteAccessToken, fetchProfile, fetchAccessTokens, generateAccessToken, saveProfile, fetchLicenseKey, saveLicenseKey, signOut } from '../../actions/index';
import { ValidLicenseConfirmDialog } from '../../shared/dialog/valid-license-confirm/valid-license-confirm.dialog';
import { Router } from '@angular/router';
import config from '../../config';
import { Observable } from 'rxjs';
import { filter, tap, map } from 'rxjs/operators';

@Component({
  template: require('./profile.component.html')
})
export class ProfileComponent implements OnInit, OnDestroy {

  isLicenseValid$: Observable<boolean> | null;
  licenseError$: Observable<string> | null;
  expirationDate$: Observable<string> | null;
  licenseKey$: Observable<string> | null;

  private allSubscriptions: any[] = [];
  private currentDialogRef: MatDialogRef<ValidLicenseConfirmDialog> | null = null;

  constructor(
    private confirmDialog: MatDialog,
    private store: AppStore,
    private title: Title,
    private router: Router
  ) {
    this.title.setTitle(`My Profile | ${store.getState().app.name}`);
    this.isLicenseValid$ = this.store.observe('users.current.license').pipe(
      filter(license => !!license),
      map(license => license.isValid)
    );
    this.licenseError$ = this.store.observe('users.current.license.error');
    this.expirationDate$ = this.store.observe('users.current.license.expirationDate');
    this.licenseKey$ = this.store.observe('users.current.license.licenseKey');
  }

  ngOnInit() {
    this.fetch();

    // if (this.config.is_saas) {
    //   // If the user is on /profile, we need to check license state
    //   this.store.dispatch(fetchLicenseKey(this.token));
    //   console.log('Fetching license key for profile component');
    //   console.log(this.isLicenseValid$.pipe( filter(isValid => isValid !== null)));
    //   // Listen for license state changes
    //   this.allSubscriptions.push(
    //     this.isLicenseValid$.pipe(
    //       filter(isValid => isValid !== null), // Ensure we only react to valid state changes
    //       tap(isValid => {
    //         if (!isValid) {
    //           if (!this.currentDialogRef) {
    //             this.openAndSetupDialog();
    //           } else if (isValid) {
    //             if (this.currentDialogRef) {
    //               this.currentDialogRef.close();
    //               this.currentDialogRef = null;
    //             }
    //           }
    //         }
    //       })
    //     )
    //   );
    // }
  }

  // Extract dialog logic to a method
  // checkAndShowLicenseDialog() {
  //   const { licenseKey, expirationDate, saveLicenseKeyErrorMessage, licenseFetchInProgress } =
  //     this.store.getState().users.current.license;
  //   if (licenseFetchInProgress) {
  //     return;
  //   }

  //   const isExpired = expirationDate ? new Date(expirationDate) < new Date() : false;
  //   let errorMsg = '';
  //   let showDialog = false;
  //   let mode = 'add';

  //   if (expirationDate && isExpired) {
  //     errorMsg = 'Your license has expired. Re-enter a new license key to download packages';
  //     this.licenseValidationMessage = errorMsg;
  //     showDialog = true;
  //   } else if (!licenseKey && this.token) {
  //     errorMsg = saveLicenseKeyErrorMessage || '';
  //     this.licenseValidationMessage = '';
  //     showDialog = true;
  //   } else if (licenseKey && (!expirationDate || !isExpired)) {
  //     this.licenseValidationMessage = expirationDate ? `License valid till ${expirationDate}` : '';
  //     if (this.dialogRef) {
  //       this.dialogRef.close();
  //       this.dialogRef = null;
  //     }
  //     return;
  //   }

  //   // Always close any open dialog before opening a new one if showDialog is true
  //   if (showDialog) {
  //     if (!this.dialogRef) {
  //       this.openAndSetupDialog(errorMsg, mode);
  //     } else {
  //       // Always update error message if dialog is open
  //       this.dialogRef.componentInstance.setErrorMessage(errorMsg);
  //     }
  //   }
  // }

  // Opens and sets up the license dialog for entering or updating a license key.
  // - Only opens if on /profile route.
  // - Sets error message if provided.
  // - Handles proceed action: validates input and saves license key.
  // - Ensures dialog cannot be closed unless a valid license is entered (in 'add' mode).
  // - If dialog is closed without a valid license, reopens dialog if still on /profile and license is missing.
  openAndSetupDialog(mode: string = 'add') {

    this.currentDialogRef = this.confirmDialog.open(ValidLicenseConfirmDialog, {
      width: '480px',
      disableClose: mode === 'add' ? true : false, // Prevent closing by clicking outside
      data: {
        heading: 'A Valid key is required for viewing and downloading the packages on the builder.',
        body: ``,
        action: `Proceed`,
        mode: mode,
      }
    });
    if (this.licenseError$) {
      const sub = this.licenseError$.subscribe((msg: string) => {
        this.currentDialogRef?.componentInstance.setErrorMessage(msg || '');
      });
      this.allSubscriptions.push(sub);
    } else {
      this.currentDialogRef.componentInstance.setErrorMessage('');
    }
    this.allSubscriptions.push(
      this.currentDialogRef.afterClosed().subscribe((result) => {
        this.currentDialogRef = null;
    }));

    this.currentDialogRef.componentInstance.proceed = () => {
      this.currentDialogRef?.componentInstance.setErrorMessage('');
      const licenseKey = this.currentDialogRef?.componentInstance.licenseKey || '';
      // Validate license key input
      // If mode is 'add', ensure license key is not empty
      // If mode is 'update', allow empty input to update the key
      if (licenseKey && licenseKey.trim().length > 0) {
        this.saveLicenseKeyToBackend({ licenseKey });
      } else {
        // If license key is empty, show error message
        this.currentDialogRef?.componentInstance.setErrorMessage('Please enter a valid license key.');
      }
    };
    // this.allSubscriptions.push(dialogRef.afterClosed().subscribe(() => {
    //   if (this.dialogRef === dialogRef) {
    //     this.dialogRef = null;
    //   }
    //   // If dialog was closed without a valid license, reopen if still on /profile
    //   if (this.router.url.startsWith('/profile')) {
    //     this.checkAndShowLicenseDialog();
    //   }
    // }));
  }

  ngOnDestroy() {
    // Guard against missing unsubscribe method
    this.allSubscriptions.forEach(sub => sub && typeof sub.unsubscribe === 'function' && sub.unsubscribe());
    this.currentDialogRef = null;
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
    this.openAndSetupDialog('update');
  }

  get licenseKey() {
    return this.store.getState().users.current.license?.licenseKey ?? null;
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
