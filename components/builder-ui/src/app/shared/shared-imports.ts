import { CommonModule } from '@angular/common';
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { RouterModule } from '@angular/router';

// Material modules
import { provideAnimations } from '@angular/platform-browser/animations';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { MatDialogModule } from '@angular/material/dialog';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatIconModule } from '@angular/material/icon';
import { MatInputModule } from '@angular/material/input';
import { MatListModule } from '@angular/material/list';
import { MatMenuModule } from '@angular/material/menu';
import { MatProgressBarModule } from '@angular/material/progress-bar';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatSelectModule } from '@angular/material/select';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatSnackBarModule } from '@angular/material/snack-bar';
import { MatTableModule } from '@angular/material/table';
import { MatTabsModule } from '@angular/material/tabs';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatTooltipModule } from '@angular/material/tooltip';

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

// Export all Material modules
export const MATERIAL_MODULES = [
  MatButtonModule,
  MatCardModule,
  MatCheckboxModule, 
  MatDialogModule,
  MatFormFieldModule,
  MatIconModule,
  MatInputModule,
  MatListModule,
  MatMenuModule,
  MatProgressBarModule,
  MatProgressSpinnerModule,
  MatSelectModule,
  MatSidenavModule,
  MatSnackBarModule,
  MatTableModule,
  MatTabsModule,
  MatToolbarModule,
  MatTooltipModule
];

// Export all shared components
export const SHARED_COMPONENTS = [
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

// Export all shared directives
export const SHARED_DIRECTIVES = [
  ClickOutsideDirective,
  AutoFocusDirective,
  SkeletonDirective
];

// Export all shared pipes
export const SHARED_PIPES = [
  TruncatePipe,
  FileSizePipe,
  TimeAgoPipe
];

// Common imports used across the application
export const COMMON_IMPORTS = [
  CommonModule,
  FormsModule,
  ReactiveFormsModule,
  RouterModule,
  ...MATERIAL_MODULES,
  ...SHARED_COMPONENTS,
  ...SHARED_DIRECTIVES,
  ...SHARED_PIPES
];
