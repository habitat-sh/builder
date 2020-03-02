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
import { GitHubApiClient } from '../client/github-api';
import { Browser } from '../browser';

export const CLEAR_GITHUB_INSTALLATIONS = 'CLEAR_GITHUB_INSTALLATIONS';
export const CLEAR_GITHUB_REPOSITORIES = 'CLEAR_GITHUB_REPOSITORIES';
export const POPULATE_GITHUB_INSTALLATIONS = 'POPULATE_GITHUB_INSTALLATIONS';
export const POPULATE_GITHUB_REPOSITORIES = 'POPULATE_GITHUB_REPOSITORIES';
export const POPULATE_GITHUB_USER_DATA = 'POPULATE_GITHUB_USER_DATA';

export function fetchGitHubInstallations(username: string) {
  const token = Browser.getCookie('oauthToken');

  return dispatch => {
    const client = new GitHubApiClient(token);
    dispatch(clearGitHubInstallations());

    client.getUserInstallations(username)
      .then((results) => {
        dispatch(populateGitHubInstallations(results));
      })
      .catch((error) => {
        console.error(error);
      });
  };
}

export function fetchGitHubRepositories(installationID: number) {
  const token = Browser.getCookie('oauthToken');

  return dispatch => {
    const client = new GitHubApiClient(token);
    dispatch(clearGitHubRepositories());

    client.getAllUserInstallationRepositories(installationID)
      .then((results) => {
        dispatch(populateGitHubRepositories(results));
      })
      .catch((error) => {
        console.error(error);
      });
  };
}

export function clearGitHubInstallations() {
  return {
    type: CLEAR_GITHUB_INSTALLATIONS
  };
}

function populateGitHubInstallations(payload) {
  return {
    type: POPULATE_GITHUB_INSTALLATIONS,
    payload,
  };
}

export function clearGitHubRepositories() {
  return {
    type: CLEAR_GITHUB_REPOSITORIES
  };
}

function populateGitHubRepositories(payload) {
  return {
    type: POPULATE_GITHUB_REPOSITORIES,
    payload,
  };
}
