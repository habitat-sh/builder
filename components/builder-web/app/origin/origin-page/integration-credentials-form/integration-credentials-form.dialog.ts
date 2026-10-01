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
import { MAT_DIALOG_DATA, MatDialogRef } from '@angular/material/dialog';
import { AppStore } from '../../../app.store';

export interface Credentials {
  name: string;
  username: string;
  password: string;
  registry_url: string;
}

export class Credentials implements Credentials {
  name: string;
  username: string = '';
  password: string = '';
  registry_url: string;
}

@Component({
  standalone: false,
  selector: 'hab-integration-credentials-dialog',
  templateUrl: './integration-credentials-form.dialog.html'
})
export class IntegrationCredentialsFormDialog {
  model: Credentials = new Credentials;

  constructor(
    public dialogRef: MatDialogRef<IntegrationCredentialsFormDialog>,
    @Inject(MAT_DIALOG_DATA) public data: any,
    private store: AppStore
  ) {
    this.model.name = data.name;
    this.model.username = data.username;
    this.model.registry_url = data.registry_url;
  }

  get token() {
    return this.store.getState().session.token;
  }

  labelFor(field) {
    return {
      docker: {
        type: 'Docker Hub',
        username: 'Docker Hub Username',
        password: 'Docker Hub Password'
      },
      amazon: {
        type: 'Amazon Container Registry',
        url: 'Registry URL',
        username: 'IAM Access Key ID',
        password: 'IAM Secret Access Key'
      },
      azure: {
        type: 'Azure Container Registry',
        url: 'Server URL',
        username: 'Service Principal ID',
        password: 'Service Principal Password'
      }
    }[this.data.type][field];
  }

  onNoClick(): void {
    this.dialogRef.close();
  }

  onSubmit() {
    this.dialogRef.close(this.model);
  }

  placeholderFor(field) {
    return {
      docker: {
        username: 'Username',
        password: 'Password'
      },
      amazon: {
        url: 'https://aws-account-id.dkr.ecr.region.amazonaws.com',
        username: 'Access Key ID',
        password: 'Secret Access Key'
      },
      azure: {
        url: 'yourserver.azurecr.io',
        username: 'Principal ID',
        password: 'Principal Password'
      }
    }[this.data.type][field];
  }

  close() {
    this.dialogRef.close();
  }
}
