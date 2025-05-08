import { Component, OnInit } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet],
  template: `<router-outlet></router-outlet>`,
  styles: []
})
export class AppComponent implements OnInit {
  title = 'Habitat Builder';

  constructor(
    private meta: Meta,
    private titleService: Title
  ) {}

  ngOnInit() {
    // Set document title
    this.titleService.setTitle(this.title);
    
    // Add theme-color meta tag for browsers
    this.meta.addTag({ name: 'theme-color', content: '#3292bf' });
    
    // Add description meta tag
    this.meta.addTag({ 
      name: 'description', 
      content: 'Habitat Builder: A platform for building, deploying, and managing applications with native integration to Chef Habitat.'
    });
  }
}
