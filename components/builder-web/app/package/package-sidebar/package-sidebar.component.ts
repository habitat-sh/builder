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

import { Component, Input } from '@angular/core';
import { AppStore } from '../../app.store';
import { submitJob } from '../../actions/index';
import { targetFrom, targets, latestBase } from '../../util';

@Component({
    selector: 'hab-package-sidebar',
    template: require('./package-sidebar.component.html')
})
export class PackageSidebarComponent {
    @Input() origin: string;
    @Input() name: string;
    @Input() target: string;
    @Input() building: boolean = false;
    @Input() isOriginMember: boolean = false;
    @Input() isNewProject: boolean = false;
    @Input() hasPlan: boolean = false;
    latestBase: string = latestBase;

    constructor(private store: AppStore) { }

    build() {
        let token = this.store.getState().session.token;
        if (this.isNewProject) {
            targets.forEach(target => this.store.dispatch(submitJob(this.origin, this.name, target.id, token)));
        } else {
            this.store.dispatch(submitJob(this.origin, this.name, this.target, token));
        }
    }

    get buildButtonLabel() {
        return this.building ? 'Build pending' :
            this.isNewProject ? 'Build latest versions' : `Build latest ${this.platform.name} version`;
    }

    get buildButtonAriaLabel() {
        return this.building ? 'Build pending' :
            this.isNewProject ? 'Build latest versions' : `Build latest ${this.platform.name} version`;
    }

    get buildButtonDisabledMessage() {
        if (!this.isStandardPkg) {
            return `* Builder can't build the package because non-standard package building is not supported.`;
        }

        return this.targetIsMac ?
            `* Builder can't build the package because a macOS plan file is not supported yet.` :
            `* Builder can't build the package because there is no ${this.platform.name} Plan file.`;
    }

    get isStandardPkg() {
        // A package can be standard or native. That means all package versions should be of the same type.
        // It is sufficient to check the type on the latest and fetched on the initial request.
        const _type = this.store.getState().packages.latest.package_type || 'standard';
        return _type.toLowerCase() === 'standard';
    }

    get exportCommand() {
        return `hab pkg export docker ${this.origin}/${this.name}`;
    }

    get isAService() {
        return this.latestStable && this.latestStable.is_a_service;
    }

    get latestStable() {
        return this.store.getState().packages.latestInChannel.stable;
    }

    get loadingLatestStable() {
        return this.store.getState().packages.ui.latestInChannel.stable.loading;
    }

    get project() {
        return this.store.getState().projects.current;
    }

    get projectExists() {
        return this.store.getState().projects.ui.current.exists;
    }

    get runCommand() {
        return `hab start ${this.origin}/${this.name}`;
    }

    get autoBuildSetting() {
        return this.project.auto_build ? 'enabled' : 'disabled';
    }

    get repoName() {
        return (this.project.vcs_data.match(/github.com\/(.+)\.git$/) || [''])[1] || '';
    }

    get repoUrl() {
        return this.project.vcs_data.replace('.git', '');
    }

    get platform() {
        return this.store.getState().packages.currentPlatform;
    }

    get targetIsMac() {
        return this.target === 'x86_64-darwin' || this.target === 'aarch64-darwin';
    }

    get isBuildable() {
        if (!this.isStandardPkg) {
            return false;
        }

        return this.isOriginMember && this.hasPlan && !this.targetIsMac && !this.building;
    }

    get packageSettings() {
        return this.store.getState().packages.currentSettings;
    }

    get defaultVisibility() {
        return this.store.getState().origins.current.default_package_visibility;
    }

    get visibility() {
        return this.packageSettings ? this.packageSettings.visibility : this.defaultVisibility;
    }

    nameFrom(platform) {
        const target = targetFrom('id', platform);
        return target ? target.name : '';
    }

    get currentBase() {
        return this.store.getState().packages.latestInChannel[this.latestBase];
    }

    get loadingCurrentBase() {
        return this.store.getState().packages.ui.latestInChannel[this.latestBase].loading;
    }

    get isBaseChannelExist() {
        const channelExist = this.store.getState().origins.current.channels.find(({ name }) => {
            return name === this.latestBase;
        });

        return channelExist?.name === this.latestBase ? true : false;
    }

    shouldRenderDeprecation(): boolean {
        let state = this.store.getState();
        return state.features.enableBase;
    }
}
