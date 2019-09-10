describe('Package Detail', () => {

  const breadcrumbNav = () => cy.get('hab-package-breadcrumbs');

  const breadcrumbNavLinks = () => breadcrumbNav().find('a');

  const tabNav = () => cy.get('nav.tabs');

  const tabNavLinks = () => tabNav().find('a');

  const pkgManifest = () => cy.get('.package-manifest');

  const pkgVersions = () => cy.get('.package-versions-component');

  const pkgBuildHelp = () => cy.get('.package-latest-component .none');

  const platformNav = () => cy.get('nav.platform-options');

  const platformNavLinks = () => platformNav().find('a');

  const buildBtn = () => cy.get('button.build');

  beforeEach(() => {
    cy.server();

    cy.fixture('package-detail/user-origins').as('userOrigins');
    cy.fixture('package-detail/user-origins-empty').as('userOriginsEmpty');
    cy.fixture('package-detail/latest-linux').as('pkgLatestLinux');
    cy.fixture('package-detail/project').as('pkgProject');
    cy.fixture('package-detail/versions-all').as('pkgVersionsAll');
    cy.fixture('package-detail/versions-empty').as('pkgVersionsEmpty');

    cy.route('GET', '/v1/user/origins', '@userOriginsEmpty');
    cy.route('GET', '/v1/depot/pkgs/core/cacerts/latest?target=x86_64-linux', '@pkgLatestLinux');
    cy.route('GET', '/v1/projects/core/cacerts', '@pkgProject');
    cy.route('GET', '/v1/depot/pkgs/core/cacerts/versions', '@pkgVersionsAll');

    cy.setSession();
  });

  it('displays breadcrumb links', () => {
    cy.visit('/#/pkgs/core/cacerts');

    breadcrumbNav().should('be.visible');
    breadcrumbNavLinks().should('have.length', 2);
    breadcrumbNavLinks().eq(0).should('contain', 'core');
    breadcrumbNavLinks().eq(1).should('contain', 'cacerts');
  });

  it('displays tab links', () => {
    cy.visit('/#/pkgs/core/cacerts');

    tabNav().should('be.visible');
    tabNavLinks().should('not.have.length', 0);
  });

  describe('when viewing as member of the pkg origin', () => {

    beforeEach(() => {
      cy.route('GET', '/v1/user/origins', '@userOrigins');
    });

    it('displays all tab links', () => {
      cy.visit('/#/pkgs/core/cacerts/latest');

      tabNav().should('be.visible');
      tabNavLinks().should('have.length', 4);
      tabNavLinks().eq(0).should('contain', 'Latest');
      tabNavLinks().eq(1).should('contain', 'Versions');
      tabNavLinks().eq(2).should('contain', 'Build Jobs');
      tabNavLinks().eq(3).should('contain', 'Settings');
    });

    it('displays a "Build latest" button', () => {
      cy.visit('/#/pkgs/core/cacerts/latest');

      buildBtn().should('be.visible');
      buildBtn().should('contain', 'Build latest version');
    });
  });

  describe('when viewing as non-member of the pkg origin', () => {
    beforeEach(() => {
      cy.route('GET', '/v1/user/origins', '@userOriginsEmpty');
    });

    it('displays only "Latest" and "Versions" tab links', () => {
      cy.visit('/#/pkgs/core/cacerts/latest');

      tabNav().should('be.visible');
      tabNavLinks().should('have.length', 2);
      tabNavLinks().eq(0).should('contain', 'Latest');
      tabNavLinks().eq(1).should('contain', 'Versions');
    });
  });

  describe('tab content for "Latest"', () => {

    it('displays the package manifest', () => {
      cy.visit('/#/pkgs/core/cacerts/latest');

      pkgManifest().should('be.visible');
      pkgManifest().should('contain', 'Maintainer');
      pkgManifest().should('contain', 'Version');
      pkgManifest().should('contain', 'Release');
      pkgManifest().should('contain', 'Target');
    });

    describe('when multiple platforms are supported', () => {

      it('displays a platform selector', () => {
        cy.visit('/#/pkgs/core/cacerts/latest');

        platformNav().should('be.visible');
        platformNavLinks().should('have.length', 3);
        platformNavLinks().eq(0).should('contain', 'Linux');
        platformNavLinks().eq(1).should('contain', 'Linux 2');
        platformNavLinks().eq(2).should('contain', 'Windows');
      });
    });

    describe('when viewing a newly-created project', () => {
      beforeEach(() => {
        cy.route('GET', '/v1/depot/pkgs/core/cacerts/versions', '@pkgVersionsEmpty');
        cy.route({ route: 'GET', url: '/v1/depot/pkgs/core/cacerts/latest?target=x86_64-linux', status: 404 });
      });

      it('does not display a platform selector', () => {
        cy.visit('/#/pkgs/core/cacerts/latest');

        platformNav().should('not.be.visible');
      });

      it('does not display a package manifest', () => {
        cy.visit('/#/pkgs/core/cacerts/latest');

        pkgManifest().should('not.be.visible');
      });

      it('displays help text for creating artifacts', () => {
        cy.visit('/#/pkgs/core/cacerts/latest');

        pkgBuildHelp().should('be.visible');
        pkgBuildHelp().should('contain', 'There are two ways to add .hart files');
      });

      describe('and when viewing as member of the pkg origin', () => {
        beforeEach(() => {
          cy.route('GET', '/v1/user/origins', '@userOrigins');
        });

        it('displays build button as "Build latest versions"', () => {
          cy.visit('/#/pkgs/core/cacerts/latest');

          buildBtn().should('be.visible');
          buildBtn().should('contain', 'Build latest versions');
        });
      });
    });
  });

  describe('tab content for "Versions"', () => {

    it('displays a list of versions', () => {
      cy.visit('/#/pkgs/core/cacerts/versions');

      pkgVersions().should('be.visible');
      pkgVersions().should('contain', 'Version');
      pkgVersions().should('contain', 'Releases');
      pkgVersions().should('contain', 'Updated');
      pkgVersions().should('contain', 'Platforms');
    });
  });
});
