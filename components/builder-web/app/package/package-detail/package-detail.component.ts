// Copyright (c) 2016-2023 Chef Software Inc. and/or applicable contributors
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

import { Component, Input } from '@angular/core';

import { AppStore } from '../../app.store';
import { parseDate, targetFrom } from '../../util';
import { demotePackage, promotePackage } from '../../actions/index';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { MatDialog } from '@angular/material';
import { PromoteConfirmDialog } from '../../shared/dialog/promote-confirm/promote-confirm.dialog';

@Component({
  selector: 'hab-package-detail',
  template: require('./package-detail.component.html')
})
export class PackageDetailComponent {
  @Input() package: any;

  updating = true;
  private _channels: any;

  constructor(
    private store: AppStore,
    private confirmDialog: MatDialog,
  ) {
   }

  @Input() set channels(channels: any) {
    const channelsMap = (channels || []).reduce((acc, cur) => {
      acc[cur.name] = cur;
      return acc;
    }, {});

    this._channels = channelsMap;
    this.updating = false;
  }

  get fullName() {
    const ident = this.package['ident'];
    let props = [];

    ['origin', 'name', 'version', 'release'].forEach(prop => {
      if (ident[prop]) {
        props.push(ident[prop]);
      }
    });

    return props.join('/');
  }

  titleFrom(platform) {
    const target = targetFrom('id', platform);
    return target ? target.title : '';
  }

  get memberOfOrigin() {
    return !!this.store.getState().origins.mine.find(
      origin => origin['name'] === this.package.ident.origin
    );
  }

  handleDemote(channel) {
    this.confirmDialog
    .open(SimpleConfirmDialog, {
      width: '480px',
      data: {
        heading: 'Confirm demote',
        body: `Are you sure you want to remove this package from the ${channel} channel?`,
        action: 'demote it'
      }
    })
    .afterClosed()
    .subscribe((confirmed) => {
      if (confirmed) {
        this.updating = true;
        let p = this.package.ident;
        let token = this.store.getState().session.token;
        this.store.dispatch(demotePackage(p.origin, p.name, p.version, p.release, this.package.target, channel, token));
      }
    });
  }

  handlePromote(channel) {
    const filteredAllChannel = this.getAllChannel(channel);
    this.confirmDialog
    .open(PromoteConfirmDialog, {
      width: '480px',
      data: {
        heading: 'Confirm promote',
        body: `Select channel to promote. Promoted artifact will be added to the selected channel.`,
        channelList: filteredAllChannel,
        action: 'Promote'
      }
    })
    .afterClosed()
    .subscribe(({confirmed, selectedChannel}) => {
      if (confirmed) {
        this.updating = true;
        let token = this.store.getState().session.token;
        this.store.dispatch(
          promotePackage(this.package.ident.origin, this.package.ident.name, this.package.ident.version, this.package.ident.release, this.package.target, selectedChannel.name, token)
        );
      }
    });
  }

  getAllChannel(currentChannel) {
    return this.store.getState().origins.current.channels.filter((channel) => {
      return channel.name !== 'unstable' && channel.name !== currentChannel;
    });
  }

  hasChannels() {
    return (this.package.channels || []).length > 0;
  }

  canShowDemote(channel) {
    return this.memberOfOrigin && channel !== 'unstable';
  }

  canShowPromote(channel, pkg) {
    return this.promotable(pkg) && channel === 'unstable';
  }

  promotable(pkg) {
    return this.memberOfOrigin &&
      pkg.channels.length > 0 &&
      pkg.channels.indexOf('stable') === -1;
  }

  releaseToDate(release) {
    return parseDate(release);
  }

  promotedDate(channel) {
    const chan = this._channels[channel] || {};
    return chan.promoted_at || chan.created_at;
  }

  toDisplaySize(size: number) {
    return this.formatSize(size);
  }

  private formatSize(bytes: number) {
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const sizes = ['bytes', 'KB', 'MB', 'GB'];

    return parseFloat((bytes / Math.pow(1024, i)).toFixed(2)) + ' ' + sizes[i];
  }
}
