describe('Search', () => {

  const results = () => cy.get('.results-component li');
  const summary = () => cy.get('.more');
  const moreLink = () => cy.get('.more a');
  const none = () => cy.get('.results-component .none');

  beforeEach(() => {
    cy.server();

    cy.fixture(`search/page-1`).as('page1');
    cy.fixture(`search/page-2`).as('page2');
    cy.fixture(`search/param`).as('search');
    cy.fixture('search/none').as('none');

    cy.route('GET', `/v1/depot/pkgs/core?range=0&distinct=true`, '@page1');
    cy.route('GET', `/v1/depot/pkgs/core?range=50&distinct=true`, '@page2');
    cy.route('GET', `/v1/depot/pkgs/search/ac?range=0&distinct=true`, '@search');
    cy.route('GET', `/v1/depot/pkgs/search/nope?range=0&distinct=true`, '@none');
  });

  describe('given an origin in the URL path', () => {

    beforeEach(() => {
      cy.visit('/#/pkgs/core');
    });

    it('renders the first page of results from the that origin', () => {
      results().should('have.length', 50);
    });
  });

  describe('given a querystring param', () => {

    beforeEach(() => {
      cy.visit('/#/pkgs/core;q=ac');
    });

    it('renders results based on that param', () => {
      results().should('have.length', 2);
    });
  });

  describe('when more than one page of results exists', () => {

    beforeEach(() => {
      cy.visit('/#/pkgs/core');
    });

    it('renders the first page', () => {
      results().should('have.length', 50);
      summary().should('contain', 'Showing 50 of 690 packages.');
      moreLink().should('contain', 'Load 50 more.');
    });

    describe('and I click to load more', () => {

      beforeEach(() => {
        cy.get('.more a').click();
      });

      it('renders the second page', () => {
        results().should('have.length', 100);
        summary().should('contain', 'Showing 100 of 690 packages.');
        moreLink().should('contain', 'Load 50 more.');
      });
    });
  });

  describe('when no results are found', () => {

    beforeEach(() => {
      cy.visit('/#/pkgs/core;q=nope');
    });

    it('shows a not-found message', () => {
      results().should('have.length', 0);
      none().should('contain', 'No packages found.');
    });
  });
});
