import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { RouterModule } from '@angular/router';
import { MaterialModule } from './material.module';

// Components
import { ButtonComponent } from './components/button/button.component';
import { CardComponent } from './components/card/card.component';
import { AlertComponent } from './components/alert/alert.component';
import { LoadingSpinnerComponent } from './components/loading-spinner/loading-spinner.component';
import { DataTableComponent } from './components/data-table/data-table.component';
import { InputComponent } from './components/input/input.component';
import { DialogComponent } from './components/dialog/dialog.component';
import { PaginationComponent } from './components/pagination/pagination.component';
import { FormFieldComponent } from './components/form-field/form-field.component';
import { SelectComponent } from './components/select/select.component';
import { FileUploadComponent } from './components/file-upload/file-upload.component';
import { BreadcrumbsComponent } from './components/breadcrumbs/breadcrumbs.component';

// Directives
import { ClickOutsideDirective } from './directives/click-outside.directive';
import { AutoFocusDirective } from './directives/auto-focus.directive';
import { SkeletonDirective } from './directives/skeleton.directive';

// Pipes
import { TruncatePipe } from './pipes/truncate.pipe';
import { FileSizePipe } from './pipes/file-size.pipe';
import { TimeAgoPipe } from './pipes/time-ago.pipe';

// Models - no need to import, they're interfaces
// './models/package.model';
// './models/origin.model';
// './models/build.model';

// Services
import { PackageService } from './services/package.service';
import { OriginService } from './services/origin.service';
import { BuildService } from './services/build.service';

// State
import { PackageState } from './state/package.state';
import { OriginState } from './state/origin.state';
import { BuildState } from './state/build.state';

// Guards
import { authGuard } from './guards/auth.guard';
import { guestGuard } from './guards/guest.guard';
import { adminGuard } from './guards/admin.guard';
import { originMemberGuard } from './guards/origin-member.guard';

// List of components, directives, and pipes exported by this module
const sharedComponents = [
  ButtonComponent,
  CardComponent,
  AlertComponent,
  LoadingSpinnerComponent,
  DataTableComponent,
  InputComponent,
  DialogComponent,
  PaginationComponent,
  FormFieldComponent,
  SelectComponent,
  FileUploadComponent,
  BreadcrumbsComponent
];

const sharedDirectives = [
  ClickOutsideDirective,
  AutoFocusDirective,
  SkeletonDirective
];

// Guards don't need to be exported, they're functional

// Services are provided at root level
const sharedServices = [
  PackageService,
  OriginService,
  BuildService
];

// State is provided at root level
const sharedState = [
  PackageState,
  OriginState,
  BuildState
];

const sharedPipes = [
  TruncatePipe,
  FileSizePipe,
  TimeAgoPipe
];

@NgModule({
  declarations: [
    ...sharedComponents,
    ...sharedDirectives,
    ...sharedPipes
  ],
  imports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    RouterModule,
    MaterialModule
  ],
  exports: [
    CommonModule,
    FormsModule,
    ReactiveFormsModule,
    RouterModule,
    MaterialModule,
    ...sharedComponents,
    ...sharedDirectives,
    ...sharedPipes
  ],
  providers: [
    ...sharedServices,
    ...sharedState
  ]
})
export class SharedModule { }
