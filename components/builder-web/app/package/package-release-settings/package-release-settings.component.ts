import { Component, OnDestroy, ChangeDetectorRef } from '@angular/core';
import { MatDialog } from '@angular/material/dialog';
import { PackageReleaseVisibilityDialog } from '../package-release-visibility-dialog/package-release-visibility.dialog';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { AppStore } from '../../app.store';
import { setPackageReleaseVisibility } from '../../actions/packages';

@Component({
  standalone: false,
  templateUrl: './package-release-settings.component.html'
})
export class PackageReleaseSettingsComponent implements OnDestroy {
  visibility: string;

  prevVisibility: string;

  private isDestroyed$: Subject<boolean> = new Subject();
  private _storeUnsub: (() => void) | null = null;

  constructor(private store: AppStore, private confirmDialog: MatDialog, private cdr: ChangeDetectorRef) {
    this.store.observe('packages.current.visibility')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(visibility => this.visibility = visibility);

    this._storeUnsub = this.store.subscribe(() => this.cdr.detectChanges());
  }

  ngOnDestroy() {
    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
    if (this._storeUnsub) { this._storeUnsub(); }
  }

  get token() {
    return this.store.getState().session.token;
  }

  get package() {
    return this.store.getState().packages.current;
  }

  handleSettingChange(setting: string) {
    this.prevVisibility = this.visibility;
    this.visibility = setting;

    if (setting === 'private') {
      this.confirmSettingChange();
    } else {
      this.saveSettingChange();
    }
  }

  confirmSettingChange() {
    this.confirmDialog
      .open(PackageReleaseVisibilityDialog, { width: '480px', data: { visibility: this.visibility, package: this.package } })
      .beforeClosed()
      .subscribe(confirmed => {
        if (confirmed) {
          this.saveSettingChange();
        } else {
          this.cancelSettingChange();
        }
      });
  }

  cancelSettingChange() {
    this.visibility = this.prevVisibility;
  }

  saveSettingChange() {
    const { origin, name, version, release } = this.package.ident;
    this.store.dispatch(setPackageReleaseVisibility(origin, name, version, release, this.visibility, this.token));
  }
}
