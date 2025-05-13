import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { By } from '@angular/platform-browser';

import { SearchResultsComponent } from './search-results.component';
import { PackageVisibility, PackageType } from '../../../shared/models/package.model';

describe('SearchResultsComponent', () => {
  let component: SearchResultsComponent;
  let fixture: ComponentFixture<SearchResultsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        SearchResultsComponent
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(SearchResultsComponent);
    component = fixture.componentInstance;
    component.packages = [
      {
        id: '1',
        ident: {
          origin: 'core',
          name: 'nginx',
          version: '1.19.3',
          release: '20201014122127'
        },
        origin: 'core',
        visibility: PackageVisibility.Public,
        type: PackageType.Standard,
        channels: [],
        platforms: ['x86_64-linux'],
        target: {
          name: 'x86_64-linux',
          platform: 'linux',
          architecture: 'x86_64'
        },
        checksum: 'abcd1234',
        manifest: '',
        config: '',
        deps: [],
        tdeps: [],
        buildDeps: [],
        buildTdeps: [],
        exposes: [],
        createdAt: new Date(),
        updatedAt: new Date(),
        ownerId: '123'
      }
    ];
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should generate the correct route for a package with version and release', () => {
    const pkg = component.packages[0];
    const route = component.routeFor(pkg);
    expect(route).toEqual(['/pkgs', 'core', 'nginx', '1.19.3', '20201014122127']);
  });

  it('should generate the correct route for a package without version and release', () => {
    const pkg = { 
      ...component.packages[0],
      ident: { 
        ...component.packages[0].ident,
        version: undefined, 
        release: undefined 
      }
    };
    const route = component.routeFor(pkg);
    expect(route).toEqual(['/pkgs', 'core', 'nginx', 'latest']);
  });

  it('should correctly format package name by removing origin prefix', () => {
    const pkg = { 
      ...component.packages[0],
      ident: { 
        ...component.packages[0].ident,
        name: 'core/nginx' 
      }
    };
    const name = component.getPackageName(pkg);
    expect(name).toBe('nginx');
  });

  it('should not modify package name when no origin prefix is present', () => {
    const pkg = component.packages[0];
    const name = component.getPackageName(pkg);
    expect(name).toBe('nginx');
  });

  it('should generate the correct route for a package with origin prefix in name', () => {
    const pkg = { 
      ...component.packages[0],
      ident: { 
        ...component.packages[0].ident,
        name: 'core/nginx' 
      }
    };
    const route = component.routeFor(pkg);
    expect(route).toEqual(['/pkgs', 'core', 'core/nginx', '1.19.3', '20201014122127']);
  });

  it('should generate the correct package string', () => {
    const pkg = component.packages[0];
    const pkgStr = component.packageString(pkg);
    expect(pkgStr).toBe('core/nginx/1.19.3/20201014122127');
  });

  it('should return correct visibility class for public package', () => {
    const publicPkg = { ...component.packages[0], visibility: PackageVisibility.Public };
    expect(component.getVisibilityClass(publicPkg)).toBe('');
  });

  it('should return correct visibility class for private package', () => {
    const privatePkg = { ...component.packages[0], visibility: PackageVisibility.Private };
    expect(component.getVisibilityClass(privatePkg)).toBe('private');
  });

  it('should return correct visibility class for hidden package', () => {
    const hiddenPkg = { ...component.packages[0], visibility: PackageVisibility.Hidden };
    expect(component.getVisibilityClass(hiddenPkg)).toBe('hidden');
  });

  it('should sort channels correctly with priority for stable and unstable', () => {
    const pkg = { 
      ...component.packages[0], 
      channels: ['testing', 'stable', 'dev', 'unstable', 'prod'] 
    };
    
    const topChannels = component.getTopChannels(pkg);
    expect(topChannels.length).toBeLessThanOrEqual(3);
    expect(topChannels[0]).toBe('stable');
    expect(topChannels[1]).toBe('unstable');
  });

  it('should handle empty channels list', () => {
    const pkg = { ...component.packages[0], channels: [] };
    const topChannels = component.getTopChannels(pkg);
    expect(topChannels).toEqual([]);
  });
});
