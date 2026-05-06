import java.util.*;

public class UserService {
    private Map<String, User> users = new HashMap<>();

    public User getUser(String id) {
        User result = users.get(id);
        if (result == null) {
            debug("missing user: " + id);
        }
        return result;
    }

    public void createUser(String id, String name) {
        User user = new User(id, name);
        users.put(id, user);
        debug("created user: " + id);
    }

    public void deleteUser(String id) {
        User removed = users.remove(id);
        if (removed != null) {
            debug("deleted user: " + id);
        }
    }

    public String describeUser(String id) {
        User user = users.get(id);
        if (user == null) {
            debug("not found: " + id);
            return "unknown";
        }
        return user.getName() + " (" + user.getId() + ")";
    }

    public int getUserCount() {
        return users.size();
    }

    private void debug(String msg) {
        System.out.println("[DEBUG] " + msg);
    }
}
