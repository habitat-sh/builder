import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { BreadcrumbsComponent } from './breadcrumbs.component';
import { NoopAnimationsModule } from '@angular/platform-browser/animations';

describe('BreadcrumbsComponent', () => {
  let component: BreadcrumbsComponent;
  let fixture: ComponentFixture<BreadcrumbsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        NoopAnimationsModule,
        BreadcrumbsComponent
      ]
    }).compileComponents();
  });

  beforeEach(() => {
    fixture = TestBed.createComponent(BreadcrumbsComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render breadcrumb items correctly', () => {
    component.items = [
      { label: 'Home', routerLink: ['/'] },
      { label: 'Origins', routerLink: ['/origins'] },
      { label: 'myorigin', routerLink: ['/origins', 'myorigin'] }
    ];
    fixture.detectChanges();
    
    const breadcrumbItems = fixture.nativeElement.querySelectorAll('.item');
    expect(breadcrumbItems.length).toBe(3);
    expect(breadcrumbItems[0].textContent).toContain('Home');
    expect(breadcrumbItems[1].textContent).toContain('Origins');
    expect(breadcrumbItems[2].textContent).toContain('myorigin');
  });

  it('should generate breadcrumbs from packageIdent', () => {
    component.packageIdent = {
      origin: 'core',
      name: 'nginx',
      version: '1.19.3',
      release: '20201014122837'
    };
    component.ngOnChanges({
      packageIdent: {
        previousValue: null,
        currentValue: component.packageIdent,
        firstChange: true,
        isFirstChange: () => true
      }
    });
    fixture.detectChanges();
    
    const breadcrumbItems = fixture.nativeElement.querySelectorAll('.item');
    expect(breadcrumbItems.length).toBe(4);
    expect(breadcrumbItems[0].textContent).toContain('core');
    expect(breadcrumbItems[1].textContent).toContain('nginx');
    expect(breadcrumbItems[2].textContent).toContain('1.19.3');
    expect(breadcrumbItems[3].textContent).toContain('20201014122837');
  });
});
