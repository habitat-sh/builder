// Copyright (c) 2016-2022 Chef Software Inc. and/or applicable contributors
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

import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer } from '@angular/platform-browser';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { RouterModule, RouterLink, RouterLinkActive } from '@angular/router';
import { MatCheckbox, MatCheckboxModule } from '@angular/material/checkbox';
import { MatDialogModule } from '@angular/material/dialog';
import { MatIconModule, MatIconRegistry } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatRadioModule, MatRadioGroup, MatRadioButton } from '@angular/material/radio';
import { MatSlideToggleModule, MatSlideToggle } from '@angular/material/slide-toggle';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatTabsModule } from '@angular/material/tabs';
import { MatButtonModule } from '@angular/material/button';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { AutoBuildSettingsComponent } from './auto-build-settings/auto-build-settings.component';
import { BreadcrumbsComponent } from './breadcrumbs/breadcrumbs.component';
import { ChannelsComponent } from './channels/channels.component';
import { CheckingInputComponent } from './checking-input/checking-input.component';
import { CopyableComponent } from './copyable/copyable.component';
import { DockerExportSettingsComponent } from './docker-export-settings/docker-export-settings.component';
import { DockerExportSettingsDialog } from './docker-export-settings/dialog/docker-export-settings.dialog';
import { DisconnectConfirmDialog } from './project-settings/dialog/disconnect-confirm/disconnect-confirm.dialog';
import { IconComponent } from './icon/icon.component';
import { DateComponent } from './date/date.component';
import { TextComponent } from './text/text.component';
import { JobCancelDialog } from './dialog/job-cancel/job-cancel.dialog';
import { JobStatusComponent } from './job-status/job-status.component';
import { JobStatusIconComponent } from './job-status-icon/job-status-icon.component';
import { JobStatusLabelComponent } from './job-status-label/job-status-label.component';
import { PackageListComponent } from './package-list/package-list.component';
import { ProjectSettingsComponent } from './project-settings/project-settings.component';
import { VisibilityIconComponent } from './visibility-icon/visibility-icon.component';
import { VisibilitySelectorComponent } from './visibility-selector/visibility-selector.component';
import { KeysPipe } from './pipes/keys.pipe';
import { SimpleConfirmDialog } from './dialog/simple-confirm/simple-confirm.dialog';
import { PromoteConfirmDialog } from './dialog/promote-confirm/promote-confirm.dialog';
import { EulaConfirmDialog } from './dialog/eula-confirm/eula-confirm.dialog';
import { BuilderEnabledGuard } from './guards/builder-enabled.guard';
import { VisibilityEnabledGuard } from './guards/visibility-enabled.guard';
import { OriginMemberGuard } from './guards/origin-member.guard';
import { SignedInGuard } from './guards/signed-in.guard';
import { JobNoticeComponent } from './job-notice/job-notice.component';
import { ValidLicenseConfirmDialog } from './dialog/valid-license-confirm/valid-license-confirm.dialog';
import { LicenseRequiredGuard } from './guards/license-required.guard';

@NgModule({
  imports: [
    CommonModule,
    FormsModule,
    MatCheckboxModule,
    MatDialogModule,
    MatIconModule,
    MatMenuModule,
    MatTabsModule,
    MatRadioModule,
    MatSlideToggleModule,
    MatTooltipModule,
    MatButtonModule,
    MatInputModule,
    MatSelectModule,
    ReactiveFormsModule,
    RouterModule,
    RouterLink,
    RouterLinkActive
  ],
  declarations: [
    AutoBuildSettingsComponent,
    BreadcrumbsComponent,
    ChannelsComponent,
    CheckingInputComponent,
    CopyableComponent,
    DisconnectConfirmDialog,
    DockerExportSettingsComponent,
    DockerExportSettingsDialog,
    IconComponent,
    DateComponent,
    TextComponent,
    JobCancelDialog,
    JobStatusComponent,
    JobStatusIconComponent,
    JobStatusLabelComponent,
    PackageListComponent,
    ProjectSettingsComponent,
    VisibilityIconComponent,
    VisibilitySelectorComponent,
    SimpleConfirmDialog,
    PromoteConfirmDialog,
    EulaConfirmDialog,
    ValidLicenseConfirmDialog,
    JobNoticeComponent,
    KeysPipe,
  ],
  exports: [
    MatDialogModule,
    MatMenuModule,
    BreadcrumbsComponent,
    ChannelsComponent,
    CheckingInputComponent,
    CopyableComponent,
    DisconnectConfirmDialog,
    DockerExportSettingsComponent,
    IconComponent,
    DateComponent,
    TextComponent,
    JobStatusComponent,
    JobStatusIconComponent,
    JobStatusLabelComponent,
    MatCheckbox,
    MatRadioGroup,
    MatRadioButton,
    MatSlideToggle,
    PackageListComponent,
    VisibilityIconComponent,
    VisibilitySelectorComponent,
    ProjectSettingsComponent,
    KeysPipe,
    SimpleConfirmDialog,
    PromoteConfirmDialog,
    JobNoticeComponent
  ],
  providers: [
    BuilderEnabledGuard,
    VisibilityEnabledGuard,
    OriginMemberGuard,
    SignedInGuard,
    LicenseRequiredGuard
  ]
})
export class SharedModule {
  constructor(private matIconRegistry: MatIconRegistry, private sanitizer: DomSanitizer) {
    matIconRegistry.addSvgIconSet(
      sanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/all.svg'),
      { viewBox: '0 0 24 24' }
    );
  }
}
