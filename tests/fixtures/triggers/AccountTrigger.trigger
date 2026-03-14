trigger AccountTrigger on Account (before insert, before update, after insert, after update) {
    if (Trigger.isBefore) {
        AccountService svc = new AccountService(new AccountRepository());
        svc.processAccounts(Trigger.new);
    }
}
