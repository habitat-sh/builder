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

import {
    AfterViewChecked, Component, ElementRef, EventEmitter, Input, OnChanges, Output,
    SimpleChanges, ViewChild
} from '@angular/core';
import { FormBuilder, FormGroup } from '@angular/forms';
import { MatDialog } from '@angular/material';
import { Record } from 'immutable';
import { DisconnectConfirmDialog } from './dialog/disconnect-confirm/disconnect-confirm.dialog';
import { DockerExportSettingsComponent } from '../../shared/docker-export-settings/docker-export-settings.component';
import { BuilderApiClient } from '../../client/builder-api';
import { AppStore } from '../../app.store';
import {
  addProject, updateProject, setProjectIntegrationSettings, deleteProject,
  fetchGitHubInstallations, fetchGitHubRepositories, fetchProject, setProjectVisibility,
  deleteProjectIntegration
} from '../../actions/index';
import config from '../../config';

@Component({
  selector: 'hab-project-settings',
  template: require('./project-settings.component.html')
})
export class ProjectSettingsComponent implements OnChanges, AfterViewChecked {
  connecting: boolean = false;
  doesFileExist: Function;
  form: FormGroup;
  activeInstallation: any;
  activeRepo: any;
  selectedInstallation: any;
  selectedRepo: string;
  selectedPath: string;

  @Input() integrations;
  @Input() name: string;
  @Input() origin: string;
  @Input() project: any;

  @Output() saved: EventEmitter<any> = new EventEmitter<any>();
  @Output() toggled: EventEmitter<any> = new EventEmitter<any>();

  @ViewChild('docker')
  docker: DockerExportSettingsComponent;

  private api: BuilderApiClient;
  private defaultPath = 'habitat/plan.sh';
  private _visibility: string;
  private _doAfterViewChecked: Function[] = [];

  constructor(
    private formBuilder: FormBuilder,
    private store: AppStore,
    private disconnectDialog: MatDialog,
    private elementRef: ElementRef
  ) {
    this.api = new BuilderApiClient(this.token);
    this.selectedPath = this.defaultPath;

    this.doesFileExist = (path) => {
      return this.api.findFileInRepo(
        this.selectedInstallation.get('installation_id'),
        this.selectedInstallation.get('org'),
        this.activeRepo.get('id'),
        this.planField.value
      );
    };
  }

  ngAfterViewChecked() {
    const f = this._doAfterViewChecked.shift();

    if (f) {
      f();
    }
  }

  ngOnChanges(changes: SimpleChanges) {
    const p = changes['project'];

    if (p && p.currentValue) {
      this.selectedRepo = p.currentValue.vcs_data;
      this.selectedPath = p.currentValue.plan_path;
      this.visibility = p.currentValue.visibility || this.visibility;
    }
  }

  get config() {
    return config;
  }

  get connectButtonLabel() {
    return this.project ? 'Update' : 'Save';
  }

  get dockerEnabled() {
    return this.dockerSettings && this.dockerSettings.size > 0;
  }

  get dockerSettings() {
    return this.store.getState().projects.current.settings;
  }

  get files() {
    return this.store.getState().gitHub.files;
  }

  get hasPrivateKey() {
    const currentOrigin = this.store.getState().origins.current;
    return currentOrigin.name === this.origin && !!currentOrigin.private_key_name;
  }

  get installations() {
    return this.store.getState().gitHub.installations;
  }

  get loadingInstallations() {
    return this.store.getState().gitHub.ui.installations.loading;
  }

  get loadingRepositories() {
    return this.store.getState().gitHub.ui.repositories.loading;
  }

  get orgs() {
    return this.store.getState().gitHub.orgs;
  }

  get planField() {
    return this.form.controls['plan_path'];
  }

  get planTemplate() {
    return {
      'origin': this.origin,
      'plan_path': this.planField.value,
      'installation_id': this.selectedInstallation.get('installation_id'),
      'repo_id': this.activeRepo.get('id')
    };
  }

  get repoField() {
    return this.form.controls['repo_path'];
  }

  get repositories() {
    return this.store.getState().gitHub.repositories;
  }

  get repoUrl() {
    if (this.selectedInstallation) {
      return `https://github.com/${this.selectedInstallation.get('full_name')}`;
    }
  }

  get token() {
    return this.store.getState().session.token;
  }

  get username() {
    return this.store.getState().users.current.gitHub.get('login');
  }

  get repoSelected() {
    return this.activeInstallation && this.activeRepo;
  }

  get validProject() {
    const planPathValid = this.planField ? this.planField.valid : false;
    const dockerValid = (this.docker && this.docker.settings.enabled) ? this.docker.settings.valid : true;
    return this.selectedInstallation && dockerValid && planPathValid;
  }

  get visibility() {
    return this._visibility || this.store.getState().origins.current.default_package_visibility || 'public';
  }

  set visibility(v: string) {
    this._visibility = v;
  }

  connect() {
    this.deselect();
    this.store.dispatch(fetchGitHubInstallations(this.username));
    this.connecting = true;
    this.toggled.emit(this.connecting);
  }

  disconnect() {
    const ref = this.disconnectDialog.open(DisconnectConfirmDialog, {
      width: '460px'
    });

    ref.afterClosed().subscribe((confirmed) => {
      if (confirmed) {
        this.store.dispatch(deleteProject(this.project.name, this.token));
      }
    });
  }

  iconFor(path) {
    return this.isWindows(path) ? 'windows' : 'linux';
  }

  isWindows(path) {
    return !!path.match(/\.ps1$/);
  }

  clearConnection() {
    this.connecting = false;
    this.deselect();
    this.toggled.emit(this.connecting);
    window.scroll(0, 0);
  }

  deselect() {
    this.form = this.formBuilder.group({});
    this.selectedRepo = null;
    this.activeInstallation = null;
    this.activeRepo = null;
    this.selectedInstallation = null;
    this.selectedPath = this.defaultPath;
  }

  editConnection() {
    this.clearConnection();
    this.connect();

    this.selectedPath = this.project.plan_path;
    this.selectedRepo = this.parseGitHubUrl(this.project.vcs_data);
    const [ org, name ] = this.selectedRepo.split('/');

    // This looks a bit weird, but it allows us to scroll the selected
    // org and repo into view. What we're doing is asking to be notified
    // when orgs and repos have been loaded, and delaying execution of the
    // functions that do that scrolling until Angular's gone through a
    // rendering cycle for each of those lists individually.
    const unsubInstalls = this.store.subscribe(state => {
      const installs = state.gitHub.installations;

      if (installs.size > 0) {
        unsubInstalls();

        installs.forEach(i => {
          if (i.get('account').get('login') === org) {
            this.pickInstallation(i);

            this.doAfterViewChecked(() => {
              this.elementRef.nativeElement.querySelector('.installations .active').scrollIntoView();
            });
          }
        });
      }
    });

    const unsubRepos = this.store.subscribe(state => {
      const repos = state.gitHub.repositories;

      if (repos.size > 0) {
        unsubRepos();

        repos.forEach(repo => {
          if (repo.get('name') === name) {
            this.pickRepo(repo);

            this.doAfterViewChecked(() => {
              this.elementRef.nativeElement.querySelector('.repositories .active').scrollIntoView();
            });
          }
        });
      }
    });
  }

  next() {
    this.selectRepository(this.activeRepo);
  }

  pickInstallation(install) {
    this.activeInstallation = install;
    this.activeRepo = null;
    this.store.dispatch(fetchGitHubRepositories(install.get('id')));
  }

  pickRepo(repo) {
    this.activeRepo = repo;
  }

  saveConnection() {
    if (this.project) {
      this.store.dispatch(updateProject(this.project.name, this.planTemplate, this.token, (result) => {
        this.handleSaved(result.success, this.project.origin_name, this.project.package_name);
      }));
    }
    else {
      this.store.dispatch(addProject(this.planTemplate, this.token, (result) => {
        this.handleSaved(result.success, result.response.origin_name, result.response.package_name);
      }));
    }
  }

  selectRepository(repo) {
    setTimeout(() => {
      if (this.planField) {
        this.planField.markAsDirty();
      }
    }, 1000);

    this.selectedInstallation = Record({
      repo_id: repo.get('id'),
      app_id: this.config.github_app_id,
      installation_id: this.activeInstallation.get('id'),
      full_name: repo.get('full_name'),
      org: repo.get('owner').get('login'),
      name: repo.get('name'),
      url: repo.get('url')
    })();
  }

  settingChanged(setting) {
    this.visibility = setting;
  }

  private doAfterViewChecked(f) {
    this._doAfterViewChecked.push(f);
  }

  private handleSaved(successful, origin, name) {
    if (successful) {
      this.saveVisibility(origin, name);
      this.saveIntegration(origin, name);
      this.store.dispatch(fetchProject(origin, name, this.token, false));
      this.saved.emit({ origin: origin, name: name });
      this.clearConnection();
    }
  }

  private parseGitHubUrl(url) {
    return (url.match(/github.com\/(.+)\.git$/) || [''])[1] || '';
  }

  private saveIntegration(origin, name) {
    const settings = this.docker.settings;
    if (settings.enabled) {
      this.store.dispatch(
        setProjectIntegrationSettings(
          origin, name, settings.name, settings.settings, this.token
        )
      );
    }
    else {
      this.store.getState().projects.current.settings.map((v, k) => {
        this.store.dispatch(deleteProjectIntegration(this.origin, this.name, k, this.token));
      });
    }
  }

  private saveVisibility(origin, name) {
    this.store.dispatch(setProjectVisibility(origin, name, this.visibility, this.token));
  }
}
