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

import { Component, Inject } from '@angular/core';
import { FormGroup, FormBuilder } from '@angular/forms';
import { MatDialogRef, MAT_DIALOG_DATA } from '@angular/material';
import { AppStore } from '../../app.store';
import { createEmptyPackage } from '../../actions/index';

@Component({
  template: require('./package-create.dialog.html')
})
export class PackageCreateDialog {
  createPackageForm: FormGroup;
  isPackageNameAvailable: Function;
  maxLength = 255;

  constructor(
    private fb: FormBuilder,
    private store: AppStore,
    private ref: MatDialogRef<PackageCreateDialog>,
    @Inject(MAT_DIALOG_DATA) private data: any
  ) {
    this.createPackageForm = fb.group({});

    this.isPackageNameAvailable = packageName => {
      console.log('validate name available');
      return;
    };
  }

  cancel() {
    this.ref.close(false);
  }

  onSubmit(value) {
    console.log(value);
    this.createPackage(value.package_name);
    this.ref.close(true);
  }

  createPackage(packageName) {
    this.store.dispatch(createEmptyPackage(packageName));
  }

  // onSubmit() {
  //   if (this.data.type === 'docker') {
  //     this.store.dispatch(validateIntegrationCredentials(this.model.username, this.model.password, this.token, this.data.type));
  //     let unsubscribe;

  //     unsubscribe = this.store.subscribe(state => {
  //       const creds = state.origins.currentIntegrations.ui.creds;

  //       if (!creds.validating && creds.validated) {
  //         unsubscribe();

  //         if (creds.valid) {
  //           setTimeout(() => this.dialogRef.close(this.model), 750);
  //         }
  //       }
  //     });
  //   } else {
  //     // We can currently only validate DockerHub creds (╯︵╰,)
  //     this.dialogRef.close(this.model);
  //   }
  // }

}
