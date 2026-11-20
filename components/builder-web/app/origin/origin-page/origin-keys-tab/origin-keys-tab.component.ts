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

import { Component, OnInit, OnDestroy, ChangeDetectorRef } from '@angular/core';
import { MatDialog } from '@angular/material/dialog';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { AppStore } from '../../../app.store';
import { BuilderApiClient } from '../../../client/builder-api';
import { GenerateKeysConfirmDialog } from './dialog/generate-keys-confirm/generate-keys-confirm.dialog';
import { KeyAddFormDialog } from './key-add-form/key-add-form.dialog';
import { deleteSigningKey, fetchOriginPublicKeys, fetchMyOrigins, generateOriginKeys } from '../../../actions/index';
import { SimpleConfirmDialog } from '../../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { OriginService } from '../../origin.service';
import config from '../../../config';

@Component({
  standalone: false,
  templateUrl: './origin-keys-tab.component.html'
})
export class OriginKeysTabComponent implements OnInit, OnDestroy {
  origin: string;
  sub: Subscription;
  private _storeUnsub: (() => void) | null = null;

  constructor(
    private route: ActivatedRoute,
    private store: AppStore,
    private dialog: MatDialog,
    private originService: OriginService,
    private title: Title,
    private cdr: ChangeDetectorRef
  ) { }

  ngOnInit() {
    this.sub = this.route.parent.params.subscribe((params) => {
      this.origin = params['origin'];
      this.title.setTitle(`Origins › ${this.origin} › Keys | ${this.store.getState().app.name}`);
      this.fetchMyOrigins();
      this.fetchPublicKeys();
    });

    this._storeUnsub = this.store.subscribe(() => this.cdr.detectChanges());
  }

  ngOnDestroy() {
    if (this._storeUnsub) { this._storeUnsub(); }
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get config() {
    return config;
  }

  get memberOfOrigin() {
    return !!this.origins.find(origin => origin['name'] === this.origin);
  }

  get origins() {
    return this.store.getState().origins.mine;
  }

  get privateKey() {
    return this.originService.origin(this.origin, this.store.getState().origins.current).private_key_name;
  }

  get isOwner() {
    const { origins, users } = this.store.getState();
    return origins.current.name === this.origin && !!users.current.profile.id &&
      users.current.profile.id === origins.current.owner_id;
  }

  get publicKeys() {
    return this.store.getState().origins.currentPublicKeys;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get ui() {
    return this.store.getState().origins.ui.current;
  }

  downloadPrivateKey() {
    new BuilderApiClient(this.store.getState().session.token)
      .getSigningKey(this.origin)
      .then((response: any) => {
        response.blob().then((blob) => {
          let header = response.headers.get('content-disposition');
          let filename = header.split(`''`).slice(-1);
          this.download(blob, unescape(filename));
        });
      });
  }

  deletePrivateKey() {
    const origin = this.origin;
    const key = this.privateKey;
    if (!this.isOwner || !key) { return; }

    this.dialog.open(SimpleConfirmDialog, {
      width: '480px',
      data: {
        heading: `Delete private signing key ${key}?`,
        body: 'Builder will lose its private copy of this revision. Its public key will still verify existing packages.',
        action: 'delete key'
      }
    }).afterClosed().subscribe(confirmed => {
      if (confirmed && this.origin === origin && this.isOwner) {
        this.store.dispatch(deleteSigningKey(origin, key.slice(origin.length + 1), this.token));
      }
    });
  }

  generateKeys() {
    this.dialog.open(GenerateKeysConfirmDialog, {
      width: '480px'
    })
      .afterClosed()
      .subscribe((confirmed) => {
        if (confirmed) {
          this.store.dispatch(generateOriginKeys(this.origin, this.token));
        }
      });
  }

  openKeyAddForm(type: string) {
    this.dialog.open(KeyAddFormDialog, {
      data: { type, origin: this.origin },
      width: '480px'
    });
  }

  urlFor(key) {
    return `v1/depot${key.location}`;
  }

  private download(blob, name) {
    const msSave = (navigator as any).msSaveBlob;

    if (typeof msSave === 'function') {
      msSave(blob, name);
    }
    else {
      let href = window.URL.createObjectURL(blob);
      let a = document.createElement('a');
      a.href = href;
      a.download = name;
      a.setAttribute('style', 'display: none');
      a.onclick = (e) => { e.stopPropagation(); };
      document.body.appendChild(a);
      a.click();
      setTimeout(() => { document.body.removeChild(a); }, 100);
    }
  }

  private fetchMyOrigins() {
    if (this.token) {
      this.store.dispatch(fetchMyOrigins(this.token));
    }
  }

  private fetchPublicKeys() {
    this.store.dispatch(fetchOriginPublicKeys(this.origin, this.token));
  }
}
