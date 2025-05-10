import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { environment } from '../../../environments/environment';

import { AuthRoutingModule } from './auth-routing.module';
import { SignInComponent } from './sign-in/sign-in.component';
import { OAuthCallbackComponent } from './oauth-callback/oauth-callback.component';
import { OAuthTokenComponent } from './oauth-token/oauth-token.component';
import { MaterialModule } from '../../shared/material.module';
import { MockOAuthCallbackComponent } from './mock-oauth-callback/mock-oauth-callback.component';

// Development components
import { DevAuthComponent } from './development/dev-auth.component';
import { AuthTestingComponent } from './development/auth-testing.component';

@NgModule({
  declarations: [],
  imports: [
    CommonModule,
    FormsModule,
    AuthRoutingModule,
    MaterialModule,
    // Standalone components are imported directly in their own files
    SignInComponent,
    OAuthCallbackComponent,
    OAuthTokenComponent,
    MockOAuthCallbackComponent,
    DevAuthComponent,
    AuthTestingComponent
  ]
})
export class AuthModule { }
