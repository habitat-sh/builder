import { Component, Input, ViewChild, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatButtonModule } from '@angular/material/button';
import { MatMenuModule } from '@angular/material/menu';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { MatCalendar, MatCalendarCellClassFunction, MatDatepickerModule } from '@angular/material/datepicker';
import { MatCardModule } from '@angular/material/card';

import { DateFilter, dateFilters, getDateRange, toDateString, toDate } from '../../utils/date-util';

@Component({
  selector: 'app-date-filter',
  standalone: true,
  imports: [
    CommonModule,
    MatButtonModule,
    MatMenuModule,
    MatIconModule,
    MatDividerModule,
    MatDatepickerModule,
    MatCardModule
  ],
  template: `
    <div class="event-filter">
      <div class="date-filter">
        <span class="show-label">Show</span>
        <ng-container>
          <button 
            [disabled]="showCalendar" 
            mat-raised-button 
            class="dropdown-toggle" 
            [matMenuTriggerFor]="menu">
            <span>{{ getCurrentFilterLabel() }}</span>
            <mat-icon>arrow_drop_down</mat-icon>
          </button>
          <mat-menu #menu="matMenu" [overlapTrigger]="false">
            <ng-container *ngFor="let filter of filters; let last = last">
              <button mat-menu-item (click)="filterChanged(filter)">
                {{ filter.label }}
              </button>
              <mat-divider *ngIf="!last"></mat-divider>
            </ng-container>
            <mat-divider></mat-divider>
            <button mat-menu-item (click)="fromCalendar()">
              Select from calendar...
            </button>
          </mat-menu>
          
          <div class="calendar-panel" *ngIf="showCalendar">
            <div class="calendar-header">
              <h3>Select a date range</h3>
              <button mat-icon-button (click)="closeDateRange()" aria-label="Close">
                <mat-icon>close</mat-icon>
              </button>
            </div>

            <div class="calendar-container">
              <div class="calendar-column">
                <h4>Start date: {{ getStartDate() }}</h4>
                <mat-calendar 
                  #fromDateCal
                  [selected]="fromSelected" 
                  (selectedChange)="fromSelected = $event"
                  [maxDate]="maxDate">
                </mat-calendar>
              </div>
              
              <div class="calendar-column">
                <h4>End date: {{ getEndDate() }}</h4>
                <mat-calendar 
                  [selected]="toSelected" 
                  (selectedChange)="toSelected = $event" 
                  [maxDate]="maxDate">
                </mat-calendar>
              </div>
            </div>
            
            <div class="calendar-actions">
              <button mat-button (click)="cancel()">Cancel</button>
              <button 
                mat-raised-button 
                color="primary" 
                (click)="apply()" 
                [disabled]="disabledApply()">
                Apply
              </button>
            </div>
          </div>
        </ng-container>
      </div>
    </div>
  `,
  styles: [`
    .event-filter {
      width: 100%;
    
      .date-filter {
        float: right;
        right: 0;
        position: relative;
      
        .show-label {
          display: inline-block;
          padding: 5px 0;
          padding-right: 16px;
          font-weight: 600;
        }
      
        .dropdown-toggle {
          background: #fff;
          border: 1px solid #ddd;
          cursor: pointer;
          
          mat-icon {
            margin-left: 4px;
          }
        }
      }
    }
    
    .calendar-panel {
      display: inline-block;
      position: absolute;
      right: 0;
      z-index: 100;
      background-color: white;
      border-radius: 4px;
      box-shadow: 0 5px 5px -3px rgba(0,0,0,.2), 
                  0 8px 10px 1px rgba(0,0,0,.14), 
                  0 3px 14px 2px rgba(0,0,0,.12);
      min-width: 38rem;
      min-height: 25rem;
      padding: 16px;
      margin-top: 40px;
      
      .calendar-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 16px;
        
        h3 {
          margin: 0 0 10px 16px;
          font-size: 16px;
          font-weight: 600;
          text-transform: uppercase;
          line-height: 1.2rem;
        }
      }
      
      .calendar-container {
        display: flex;
        justify-content: space-between;
        
        .calendar-column {
          width: 300px;
          
          &:first-child {
            border-right: 1px solid #eee;
            padding-right: 16px;
            float: left;
          }
          
          &:last-child {
            padding-left: 16px;
            float: right;
          }
          
          h4 {
            margin: 0 0 8px 16px;
            font-size: 14px;
            font-weight: normal;
            text-transform: uppercase;
            color: #666;
          }
        }
      }
      
      .calendar-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        margin-top: 16px;
        padding-top: 16px;
        text-align: right;
        
        button:first-child {
          margin-right: 10px;
        }
      }
    }
    
    .clearfix {
      content: "";
      clear: both;
      display: table;
    }
  `]
})
export class DateFilterComponent implements OnInit {
  @ViewChild('fromDateCal') fromDateCal!: MatCalendar<Date>;

  @Input() dateFilterChanged!: (filter: DateFilter) => void;
  @Input() currentFilter: DateFilter = dateFilters[0];
  @Input() filters: DateFilter[] = dateFilters;

  maxDate = new Date();
  fromSelected: Date | null = null;
  toSelected: Date | null = null;
  showCalendar = false;

  ngOnInit(): void {
    // Initialize the date range
    const dateRange = getDateRange(this.currentFilter);
    this.fromSelected = toDate(dateRange.fromDate);
    this.toSelected = toDate(dateRange.toDate);
  }

  getCurrentFilterLabel(): string {
    return this.currentFilter?.label || 'Last 1 Week';
  }

  filterChanged(filter: DateFilter): void {
    if (this.currentFilter?.label === filter.label) {
      return;
    }

    this.dateFilterChanged(filter);
  }

  fromCalendar(): void {
    this.showCalendar = true;
    const dateRange = getDateRange(this.currentFilter);
    this.fromSelected = toDate(dateRange.fromDate);
    this.toSelected = toDate(dateRange.toDate);

    // Need to set the active date after the calendar is rendered
    setTimeout(() => {
      if (this.fromDateCal) {
        this.fromDateCal.activeDate = this.fromSelected!;
      }
    }, 100);
  }

  closeDateRange(): void {
    this.showCalendar = false;
  }

  cancel(): void {
    this.closeDateRange();
  }

  apply(): void {
    if (!this.fromSelected || !this.toSelected) return;
    
    const fromDateStr = toDateString(this.fromSelected);
    const toDateStr = toDateString(this.toSelected);
    const filter: DateFilter = {
      label: `${fromDateStr} - ${toDateStr}`,
      type: 'custom',
      startDate: this.fromSelected,
      endDate: this.toSelected
    };

    this.dateFilterChanged(filter);
    this.closeDateRange();
  }

  disabledApply(): boolean {
    if (!this.fromSelected || !this.toSelected) return true;
    return this.fromSelected > this.toSelected;
  }

  getStartDate(): string {
    return this.fromSelected ? toDateString(this.fromSelected) : '';
  }

  getEndDate(): string {
    return this.toSelected ? toDateString(this.toSelected) : '';
  }
}
