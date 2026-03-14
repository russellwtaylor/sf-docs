/**
 * Service class for managing Order records within an account context.
 */
public class OrderService extends BaseService {
    public OrderService() {}

    public List<Order__c> getOrdersForAccount(Id accountId) {
        return [SELECT Id, Amount__c FROM Order__c WHERE AccountId = :accountId];
    }

    public void cancelOrder(Id orderId) {
        Order__c ord = [SELECT Id, Status__c FROM Order__c WHERE Id = :orderId];
        ord.Status__c = 'Cancelled';
        update ord;
    }
}
