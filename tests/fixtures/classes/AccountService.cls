/**
 * Service class for managing Account records.
 */
public with sharing class AccountService implements Queueable {
    private AccountRepository repo;
    public static Integer MAX_BATCH = 200;

    public AccountService(AccountRepository repo) {
        this.repo = repo;
    }

    public void processAccounts(List<Account> accounts) {
        for (Account acc : accounts) {
            acc.Description = 'Processed';
        }
        repo.saveAll(accounts);
    }

    public static List<Account> getActiveAccounts() {
        return [SELECT Id, Name FROM Account WHERE IsActive__c = true];
    }
}
