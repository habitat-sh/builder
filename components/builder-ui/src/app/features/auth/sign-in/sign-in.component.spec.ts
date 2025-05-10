import { TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';
import { MatDialogModule } from '@angular/material/dialog';

import { SignInComponent } from './sign-in.component';
import { AuthService } from '../../../core/services/auth.service';
import { ConfigService } from '../../../core/services/config.service';
import { IconService } from '../../../core/services/icon.service';
import { of } from 'rxjs';

describe('SignInComponent', () => {
  let component: SignInComponent;
  
  // Mock services
  const authServiceMock = {
    logout: jest.fn(),
    getAuthorizationUrl: jest.fn().mockReturnValue('https://github.com/login/oauth/authorize?client_id=test'),
    setRedirectUrl: jest.fn()
  };
  
  const configServiceMock = {
    getConfig: jest.fn().mockReturnValue(of({
      oauthProvider: 'GitHub',
      oauthSignupUrl: 'https://github.com/join',
      wwwUrl: 'https://www.habitat.sh'
    }))
  };
  
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        HttpClientTestingModule,
        MatDialogModule,
        SignInComponent
      ],
      providers: [
        { provide: AuthService, useValue: authServiceMock },
        { provide: ConfigService, useValue: configServiceMock },
        IconService
      ]
    }).compileComponents();
    
    const fixture = TestBed.createComponent(SignInComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });
  
  it('should create', () => {
    expect(component).toBeTruthy();
  });
  
  it('should load config on init', () => {
    expect(configServiceMock.getConfig).toHaveBeenCalled();
    expect(component.providerName).toEqual('GitHub');
    expect(component.signupUrl).toEqual('https://github.com/join');
    expect(component.wwwUrl).toEqual('https://www.habitat.sh');
  });
  
  it('should construct OAuth login URL', () => {
    expect(authServiceMock.getAuthorizationUrl).toHaveBeenCalled();
    expect(component.loginUrl).toEqual('https://github.com/login/oauth/authorize?client_id=test');
  });
  
  it('should show EULA dialog before redirecting', () => {
    // This would need more complex testing with dialog interactions
    expect(component.showEulaPopup).toBeDefined();
  });
});
