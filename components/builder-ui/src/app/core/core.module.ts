import { NgModule, Optional, SkipSelf } from '@angular/core';
import { CommonModule } from '@angular/common';
import { HttpClientModule } from '@angular/common/http';
import { MatSnackBarModule } from '@angular/material/snack-bar';

// Services
import { ApiService } from './services/api.service';
import { AuthService } from './services/auth.service';
import { NotificationService } from './services/notification.service';
import { LoadingService } from './services/loading.service';
import { DialogService } from './services/dialog.service';

// Interceptors
import { httpInterceptorProviders } from './interceptors';

@NgModule({
  declarations: [],
  imports: [
    CommonModule,
    HttpClientModule,
    MatSnackBarModule
  ],
  exports: [],
  providers: [
    ApiService,
    AuthService,
    NotificationService,
    LoadingService,
    DialogService,
    httpInterceptorProviders
  ]
})
export class CoreModule {
  constructor(@Optional() @SkipSelf() parentModule: CoreModule) {
    if (parentModule) {
      throw new Error('CoreModule is already loaded. Import it in the AppModule only.');
    }
  }
}
