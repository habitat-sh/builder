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

import 'whatwg-fetch';
import * as async from 'async';
import config from '../config';

export class GitHubApiClient {
  private headers;

  constructor(private token: string) {
    this.headers = {
      Accept: [
        'application/vnd.github.v3+json',
        'application/vnd.github.machine-man-preview+json'
      ],
      Authorization: 'token ' + token
    };
  }

  public getUserInstallations(username: string) {
    return new Promise((resolve, reject) => {
      fetch(`${config['github_api_url']}/user/installations`, {
        method: 'GET',
        headers: this.headers
      })
        .then(response => {
          if (response.ok) {
            response.json().then((data) => {

              resolve(
                data.installations
                  .filter(install => {
                    return install.app_id.toString() === config.github_app_id;
                  })
                  .filter(install => {
                    return install.target_type === 'Organization' ||
                      (install.target_type === 'User' && install.account.login === username);
                  })
                );
            });
          } else {
            reject(new Error(response.statusText));
          }
        })
        .catch(error => {
          reject(error);
        });
      });
    }

    public getAllUserInstallationRepositories(installID) {
      return new Promise((resolve, reject) => {
        this.getUserInstallationRepositories(installID, 1)
          .then((firstPage: any) => {
            const totalCount = firstPage.total_count;
            const thisPage = firstPage.repositories;

            if (totalCount > thisPage.length) {
              const pageCount = Math.ceil(totalCount / thisPage.length);
              let pages = [];

              for (let page = 2; page <= pageCount; page++) {
                pages.push((done) => {
                  this.getUserInstallationRepositories(installID, page)
                    .then((pageResults: any) => {
                      done(null, pageResults.repositories);
                    })
                    .catch((err) => {
                      console.error(err);
                      done(null, []);
                    });
                });
              }

              async.parallel(pages, (err, additionalPages) => {
                if (err) {
                  console.error(err);
                  resolve([]);
                }
                else {
                  additionalPages.forEach((p) => {
                    firstPage.repositories = firstPage.repositories.concat(p);
                  });

                  resolve(firstPage.repositories);
                }
              });
            }
            else {
              resolve(firstPage.repositories);
            }
          })
          .catch((err) => {
            console.error(err);
            resolve([]);
          });
      });
    }

    private getUserInstallationRepositories(installationId: string, page: number) {
      return new Promise((resolve, reject) => {
        fetch(`${config['github_api_url']}/user/installations/${installationId}/repositories?page=${page}&per_page=100`, {
          method: 'GET',
          headers: this.headers
        })
          .then(response => {
            if (response.ok) {
              resolve(response.json());
            } else {
              reject(new Error(response.statusText));
            }
          })
          .catch(error => {
            reject(error);
          });
      });
    }
  }
