// Copyright (c) 2016-2021 Chef Software Inc. and/or applicable contributors
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

import 'whatwg-fetch';
import config from '../config';
import { parseKey } from '../util';
import { AppStore } from '../app.store';
import { addNotification, signOut } from '../actions/index';
import { WARNING } from '../actions/notifications';

export enum ErrorCode {
  NotFound = 4
}
export class BuilderApiClient {
  private headers;
  private jsonHeaders;
  private urlPrefix: string;
  private store: AppStore;

  constructor(private token: string = '') {
    this.urlPrefix = 'v1';
    this.headers = token ? { 'Authorization': `Bearer ${token}` } : {};
    this.jsonHeaders = { 'Authorization': `Bearer ${token}`, 'Content-Type': 'application/json' };
    this.store = new AppStore();
  }

  public acceptOriginInvitation(invitationId: string, originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/invitations/${invitationId}`, {
        headers: this.headers,
        method: 'PUT',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteOriginInvitation(invitationId: string, originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/invitations/${invitationId}`, {
        headers: this.headers,
        method: 'DELETE',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteOriginMember(origin: string, member: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/users/${member}`, {
        headers: this.headers,
        method: 'DELETE',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public departFromOrigin(origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/depart`, {
        headers: this.headers,
        method: 'POST',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public ignoreOriginInvitation(invitationId: string, originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/invitations/${invitationId}/ignore`, {
        headers: this.headers,
        method: 'PUT',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public createOrigin(origin) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins`, {
        body: JSON.stringify(origin),
        headers: this.jsonHeaders,
        method: 'POST',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public createOriginKey(key) {
    key = parseKey(key);
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${key.uploadPath}`, {
        body: key.text,
        headers: this.headers,
        method: 'POST',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public createEmptyPackage(packageInfo) {
    const { origin, packageName } = packageInfo;

    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/settings/${origin}/${packageName}`, {
        headers: this.jsonHeaders,
        method: 'POST',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public findFileInRepo(installationId: string, owner: string, repoId: string, path: string, page: number = 1, per_page: number = 100) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/ext/installations/${installationId}/repos/${repoId}/contents/${encodeURIComponent(path)}`, {
        method: 'GET',
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public generateOriginKeys(origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/keys`, {
        method: 'POST',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getAccessTokens() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile/access-tokens`, {
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public generateAccessToken() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile/access-tokens`, {
        method: 'POST',
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteAccessToken(id: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile/access-tokens/${id}`, {
        headers: this.headers,
        method: 'DELETE',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getJob(id: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/jobs/${id}`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getJobLog(id: string, start = 0) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/jobs/${id}/log?start=${start}&color=true`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            response.json().then(reject);
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getJobs(origin: string, name: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}/jobs`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getJobGroups(origin: string, limit: number) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/pkgs/schedule/${origin}/status?limit=${limit}`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getJobGroup(id: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/pkgs/schedule/${id}?include_projects=true`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            resolve(null);
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public cancelJobGroup(id: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/jobs/group/${id}/cancel`, {
        method: 'POST',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getProject(origin: string, name: string, target: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}?target=${target}`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getProjects(origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getMyOriginInvitations() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/user/invitations`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getMyOrigins() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/user/origins`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOrigin(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOriginInvitations(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/invitations`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            response.json().then(data => {
              resolve(data['invitations']);
            });
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOriginMembers(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/users`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            response.json().then(data => {
              resolve(data['members']);
            });
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOriginPublicKeys(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/keys`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOriginSecrets(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/secret`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getOriginChannels(origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/channels/${origin}`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getProfile() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public saveProfile(profile: any) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile`, {
        headers: this.jsonHeaders,
        method: 'PATCH',
        body: JSON.stringify(profile)
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public inviteUserToOrigin(username: string, origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/users/${username}/invitations`, {
        headers: this.headers,
        method: 'POST',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(true);
          } else if (response.status === 404) {
            reject(new Error(`We were unable to locate a Builder user named ${username}. Please ensure the user has signed into Builder at least once before, then send the invitation again.`));
          } else if (response.status === 409) {
            reject(new Error(`An invitation already exists for ${username}.`));
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public isOriginAvailable(name: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${name}`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          // Getting a 200 means it exists and is already taken.
          if (response.ok) {
            reject(false);
            // Getting a 404 means it does not exist and is available.
          } else if (response.status === 404) {
            resolve(true);
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public isPackageNameAvailable(origin: string, packageName: string) {
    return new Promise((resolve, reject) => {

      fetch(`${this.urlPrefix}/settings/${origin}/${packageName}`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          // Getting a 200 means it exists and is already taken.
          if (response.ok) {
            reject(false);
            // Getting a 404 means it does not exist and is available.
          } else if (response.status === 404) {
            resolve(true);
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getPackageSettings(origin: string, name: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/settings/${origin}/${name}`, {
        headers: this.headers,
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public setPackageVisibility(origin: string, name: string, setting: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/settings/${origin}/${name}`, {
        headers: this.jsonHeaders,
        method: 'PUT',
        body: JSON.stringify({ visibility: setting })
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getIntegration(origin: string, type: string, name: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/integrations/${type}/${name}`, {
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getIntegrations(originName: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/integrations`, {
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public setIntegration(originName: string, credentials, type: string, name: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${originName}/integrations/${type}/${name}`, {
        headers: this.jsonHeaders,
        method: 'PUT',
        body: JSON.stringify(credentials)
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getProjectIntegration(origin: string, name: string, integration: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}/integrations/${integration}/default`, {
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response.json());
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public setProjectIntegrationSettings(origin: string, name: string, integration: string, settings: any) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}/integrations/${integration}/default`, {
        headers: this.jsonHeaders,
        method: 'PUT',
        body: JSON.stringify(settings)
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteProjectIntegration(origin: string, name: string, integration: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}/integrations/${integration}/default`, {
        headers: this.headers,
        method: 'DELETE'
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public setProjectVisibility(origin: string, name: string, setting: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/projects/${origin}/${name}/${setting}`, {
        headers: this.headers,
        method: 'PATCH'
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public setPackageReleaseVisibility(origin: string, name: string, version: string, release: string, setting: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/pkgs/${origin}/${name}/${version}/${release}/${setting}`, {
        headers: this.headers,
        method: 'PATCH'
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteIntegration(origin: string, name: string, type: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/integrations/${type}/${name}`, {
        headers: this.headers,
        method: 'DELETE',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public updateOrigin(origin: any) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin.name}`, {
        headers: this.jsonHeaders,
        method: 'PUT',
        body: JSON.stringify(origin)
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public deleteOriginSecret(origin: string, key: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/secret/${key}`, {
        headers: this.headers,
        method: 'DELETE'
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve();
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public getSigningKey(origin: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/depot/origins/${origin}/secret_keys/latest`, {
        headers: this.headers
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            resolve(response);
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public validateIntegrationCredentials(username: string, password: string, type: string, url?: string) {
    let creds = { username, password };
    if (url && url.trim() !== '') {
      creds['url'] = url.trim();
    }

    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/ext/integrations/${type}/credentials/validate`, {
        headers: this.jsonHeaders,
        method: 'POST',
        body: JSON.stringify(creds)
      })
        .then(response => {
          if (response.ok) {
            resolve();
          }
          else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => reject(error));
    });
  }

  public getLicenseKey() {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile/license`, {
        headers: this.headers,
        method: 'GET',
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            response.json().then(resolve).catch(() => resolve({}));
          } else {
            response.text().then(msg => reject(new Error(msg)));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  public saveLicenseKey(licenseKey: string, accountId: string) {
    return new Promise((resolve, reject) => {
      fetch(`${this.urlPrefix}/profile/license`, {
        headers: this.jsonHeaders,
        method: 'PUT',
        body: JSON.stringify({ license_key: licenseKey, account_id: accountId })
      })
        .then(response => this.handleUnauthorized(response, reject))
        .then(response => {
          if (response.ok) {
            response.json().then(resolve).catch(() => resolve({}));
          } else {
            response.text().then(msg => reject(new Error(msg)));
          }
        })
        .catch(error => this.handleError(error, reject));
    });
  }

  private handleError(error, reject) {
    const store = this.store;
    const state = store.getState();
    store.dispatch(signOut(true, state.router.route.url));
    reject(error);

    if (state.session.token) {
      setTimeout(() => {
        store.dispatch(addNotification({
          title: 'Session Expired',
          body: 'Please sign in again.',
          type: WARNING
        }));
      }, 1000);
    }
  }

  private handleUnauthorized(response, reject) {
    if (response.status === 401) {
      throw new Error('Unauthorized');
    }

    return response;
  }
}
