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

export {
  CLEAR_JOB_LOG,
  CLEAR_JOB,
  CLEAR_JOBS,
  clearJob,
  clearJobs,
  fetchJob,
  fetchJobLog,
  fetchJobs,
  POPULATE_JOB_LOG,
  POPULATE_JOB,
  POPULATE_JOBS,
  STREAM_JOB_LOG,
  SET_JOB_LOADING,
  SET_JOBS_LOADING,
  SET_JOB_LOG_LOADING,
  SET_JOB_LOG_NOT_FOUND,
  streamJobLog,
  submitJob
} from './jobs';

export {
  cancelJobGroup,
  fetchJobGroups,
  fetchJobGroup,
  POPULATE_JOB_GROUPS,
  POPULATE_JOB_GROUP
} from './jobGroups';

export {
  LOAD_FEATURES,
  loadFeatures
} from './features';

export {
  CLEAR_GITHUB_INSTALLATIONS,
  CLEAR_GITHUB_REPOSITORIES,
  fetchGitHubInstallations,
  fetchGitHubRepositories,
  POPULATE_GITHUB_INSTALLATIONS,
  POPULATE_GITHUB_REPOSITORIES
} from './gitHub';

export {
  ADD_NOTIFICATION,
  addNotification,
  REMOVE_NOTIFICATION,
  removeNotification
} from './notifications';

export {
  authenticate,
  exchangeOAuthCode,
  LOAD_OAUTH_STATE,
  loadOAuthState,
  removeSession,
  SET_OAUTH_PROVIDER,
  loadOAuthProvider,
  SET_OAUTH_STATE,
  SET_OAUTH_TOKEN,
  setOAuthState
} from './oauth';

export {
  acceptOriginInvitation,
  CLEAR_INTEGRATION,
  CLEAR_INTEGRATIONS,
  CLEAR_MY_ORIGIN_INVITATIONS,
  CLEAR_MY_ORIGINS,
  clearIntegrationCredsValidation,
  createOrigin,
  deleteIntegration,
  deleteOriginInvitation,
  deleteOriginMember,
  deleteOriginSecret,
  fetchIntegration,
  fetchIntegrations,
  fetchMyOriginInvitations,
  fetchMyOrigins,
  fetchOrigin,
  fetchOriginInvitations,
  fetchOriginMembers,
  fetchOriginPublicKeys,
  fetchOriginSecrets,
  generateOriginKeys,
  ignoreOriginInvitation,
  inviteUserToOrigin,
  POPULATE_MY_ORIGIN_INVITATIONS,
  POPULATE_MY_ORIGINS,
  POPULATE_ORIGIN_INTEGRATION,
  POPULATE_ORIGIN_INTEGRATIONS,
  POPULATE_ORIGIN_INVITATIONS,
  POPULATE_ORIGIN_MEMBERS,
  POPULATE_ORIGIN_PUBLIC_KEYS,
  POPULATE_ORIGIN_SECRETS,
  SET_CURRENT_ORIGIN_ADDING_PRIVATE_KEY,
  SET_CURRENT_ORIGIN_ADDING_PUBLIC_KEY,
  SET_CURRENT_ORIGIN_CREATING_FLAG,
  SET_CURRENT_ORIGIN_LOADING,
  SET_CURRENT_ORIGIN,
  SET_INTEGRATION_CREDS_VALIDATION,
  SET_ORIGIN_INTEGRATION_SAVE_ERROR_MESSAGE,
  SET_ORIGIN_USER_INVITE_ERROR_MESSAGE,
  setCurrentOrigin,
  setIntegration,
  TOGGLE_ORIGIN_PICKER,
  toggleOriginPicker,
  UPDATE_ORIGIN,
  updateOrigin,
  uploadOriginPrivateKey,
  uploadOriginPublicKey,
  validateIntegrationCredentials
} from './origins';

export {
  CLEAR_CURRENT_PACKAGE_CHANNELS,
  CLEAR_LATEST_IN_CHANNEL,
  CLEAR_LATEST_PACKAGE,
  CLEAR_PACKAGE_VERSIONS,
  CLEAR_PACKAGES,
  demotePackage,
  fetchDashboardRecent,
  fetchLatestInChannel,
  fetchLatestPackage,
  fetchPackage,
  fetchPackageChannels,
  fetchPackageVersions,
  filterPackagesBy,
  getUniquePackages,
  POPULATE_DASHBOARD_RECENT,
  promotePackage,
  SET_CURRENT_PACKAGE_CHANNELS,
  SET_CURRENT_PACKAGE_VERSIONS,
  SET_CURRENT_PACKAGE,
  SET_CURRENT_PACKAGE_TARGET,
  SET_CURRENT_PACKAGE_TARGETS,
  SET_LATEST_IN_CHANNEL,
  SET_LATEST_PACKAGE,
  SET_PACKAGES_NEXT_RANGE,
  SET_PACKAGES_SEARCH_QUERY,
  SET_PACKAGES_TOTAL_COUNT,
  SET_VISIBLE_PACKAGES,
  clearPackageVersions,
  setCurrentPackage,
  setCurrentPackageTarget,
  setCurrentPackageTargets,
  setPackagesSearchQuery,
  setVisiblePackages
} from './packages';

export {
  addProject,
  CLEAR_PROJECTS,
  CLEAR_CURRENT_PROJECT,
  CLEAR_CURRENT_PROJECT_INTEGRATION,
  deleteProject,
  deleteProjectIntegration,
  fetchProject,
  fetchProjects,
  SET_CURRENT_PROJECT_INTEGRATION,
  SET_CURRENT_PROJECT,
  SET_PROJECTS,
  setCurrentProject,
  setProjectIntegrationSettings,
  setProjectVisibility,
  updateProject
} from './projects';

export {
  requestRoute,
  ROUTE_CHANGE,
  ROUTE_CHANGE_END,
  ROUTE_REQUESTED,
  routeChange,
  routeChangeEnd
} from './router';

export {
  LOAD_BLDR_SESSION_STATE,
  loadBldrSessionState,
  SET_BLDR_SESSION_TOKEN
} from './sessions';

export {
  SET_LAYOUT,
  setLayout
} from './ui';

export {
  CLEAR_ACCESS_TOKENS,
  clearAccessTokens,
  CLEAR_NEW_ACCESS_TOKEN,
  clearNewAccessToken,
  deleteAccessToken,
  setCurrentUsername,
  fetchProfile,
  fetchAccessTokens,
  generateAccessToken,
  identifyUser,
  POPULATE_ACCESS_TOKENS,
  POPULATE_NEW_ACCESS_TOKEN,
  POPULATE_PROFILE,
  saveProfile,
  SET_LOADING_ACCESS_TOKENS,
  setLoadingAccessTokens,
  SET_DELETING_ACCESS_TOKEN,
  setDeletingAccessToken,
  SET_GENERATING_ACCESS_TOKEN,
  setGeneratingAccessToken,
  SET_CURRENT_USERNAME,
  SET_PRIVILEGES,
  setPrivileges,
  SIGNING_IN,
  signingIn,
  SIGN_IN_FAILED,
  signInFailed,
  signOut,
  TOGGLE_USER_NAV_MENU,
  toggleUserNavMenu
} from './users';

// Used by redux-reset to reset the app state
export const RESET = 'RESET';

export function resetAppState() {
  return {
    type: RESET,
  };
}
