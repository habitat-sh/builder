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
import { MatDialogRef, MAT_DIALOG_DATA } from '@angular/material';

@Component({
  template: require('./package-release-visibility.dialog.html')
})
export class PackageReleaseVisibilityDialog {
  constructor(
    private ref: MatDialogRef<PackageReleaseVisibilityDialog>,
    @Inject(MAT_DIALOG_DATA) private data: any
  ) {}

  get artifactName() {
    const { origin, name, version, release } = this.data.package.ident;
    return [origin, name, version, release].join('/');
  }

  cancel() {
    this.ref.close(false);
  }

  confirm() {
    this.ref.close(true);
  }
}
