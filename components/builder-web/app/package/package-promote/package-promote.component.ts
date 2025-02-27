import { Component, Input } from '@angular/core';
import { MatDialog } from '@angular/material';
import { AppStore } from '../../app.store';
import { promotePackage } from '../../actions/index';
import { SimpleConfirmDialog } from '../../shared/dialog/simple-confirm/simple-confirm.dialog';
import { PromoteConfirmDialog } from '../../shared/dialog/promote-confirm/promote-confirm.dialog';

@Component({
    selector: 'hab-package-promote',
    template: require('./package-promote.component.html')
})
export class PackagePromoteComponent {
    @Input() origin: string;
    @Input() name: string;
    @Input() version: string;
    @Input() release: string;
    @Input() target: string;
    @Input() channels: string[];
    @Input() enabledBase;

    promoting: boolean = false;

    constructor(
        private confirmDialog: MatDialog,
        private store: AppStore
    ) { }

    prompt(evt) {
        let state = this.store.getState();
        evt.stopPropagation();

        const filteredAllChannel = this.getAllChannel();
        if (!state.features.enableBase) {
            this.confirmDialog
                .open(SimpleConfirmDialog, {
                    width: '480px',
                    data: {
                        heading: 'Confirm promote',
                        body: `Are you sure you want to promote this artifact? Doing so will add the artifact to the stable channel.`,
                        action: 'promote it'
                    }
                })
                .afterClosed()
                .subscribe((confirmed) => {
                    if (confirmed) {
                        this.promoting = true;
                        setTimeout(() => {
                            this.store.dispatch(
                                promotePackage(this.origin, this.name, this.version, this.release, this.target, 'stable', this.store.getState().session.token)
                            );
                        }, 1000);
                    }
                });
        } else {
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
                .subscribe((data) => {
                    if (data) {
                        const { confirmed, selectedChannel } = data;
                        if (confirmed && selectedChannel) {
                            this.promoting = true;
                            let token = this.store.getState().session.token;
                            this.store.dispatch(
                                promotePackage(this.origin, this.name, this.version, this.release, this.target, selectedChannel, token)
                            );
                        }
                    }
                });
        }
    }

    getAllChannel() {
        return this.store.getState().origins.current.channels.filter((channel) => {
            return channel.name !== 'unstable' && this.channels.indexOf(channel.name) === -1;
        });
    }

    get promoteText(): string {
        let state = this.store.getState();
        return state.features.enableBase ? 'Promote' : 'Promote to stable';
    }
}
