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

import { Component, Input, Output, EventEmitter } from '@angular/core';
import { MatDialog } from '@angular/material';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';

@Component({
  selector: 'hab-channels',
  template: require('./channels.component.html')
})
export class ChannelsComponent {
  @Input() channels: string[];
  @Input() canDemote: boolean = false;
  @Output() onDemote: EventEmitter<string> = new EventEmitter<string>();

  constructor(
    private confirmDialog: MatDialog,
  ) { }

  demotable(channel) {
    return this.canDemote && channel !== 'unstable';
  }

  outerClick(e) {
    e.stopPropagation();
  }

  demote(channel) {
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
          this.onDemote.emit(channel);
        }
      });
  }
}
