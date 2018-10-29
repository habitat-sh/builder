describe('The default route', () => {

  beforeEach(() => {
    cy.server();
    cy.fixture('search/some').as('some');
    cy.route('GET', '/v1/depot/pkgs/core?range=0&distinct=true', '@some');
    cy.visit('/#/');
  });

  it('renders the search view', () => {
    cy.get('header h1').should('contain', 'Search Packages');
    cy.get('.results-component li').children().should('have.length.greaterThan', 0);
  });
});
