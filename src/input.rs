use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{RwLock};
use std::hash::Hash;
pub struct InputState(RwLock<InputData>);
pub struct InputData
{
    map_press : HashMap<PressKey, bool>,

    map_key : HashMap<String, HashSet<PressKey>>,
}

pub struct KeyEventQueue
{
    events : VecDeque<(HashSet<String>, bool)>,
    possible_keys : HashSet<String>
}

impl KeyEventQueue
{
    pub fn new(possible_keys : HashSet<&str>)
        -> KeyEventQueue
    {
        KeyEventQueue {
            events: VecDeque::new(), 
            possible_keys: possible_keys.into_iter().map(|s| String::from(s)).collect()}
    }
    fn queue_key_events(&mut self, key_events : &HashSet<String>, pressed : bool)
    {
        let event_keys : HashSet<String> = self.possible_keys.intersection(key_events).cloned().collect();

        if event_keys.len() == 0 {return}

        self.events.push_back(
            (event_keys, pressed));
    }

    pub fn pop_event(&mut self)
        -> Option<(HashSet<String>, bool)>
    {
        self.events.pop_front()
    }

    pub fn pop_events(&mut self, num : usize)
        -> Vec<(HashSet<String>, bool)>
    {
        let mut events : Vec<(HashSet<String>, bool)> = Vec::new();

        for _ in 0..num
        {
            if let Some(event) = self.pop_event()
            {
                events.push(event);
            }
            else
            {
                break
            }
        }

        events
    }


}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum PressKey
{
    KeyScancode(usize),
    MouseButton(usize)
}

// An rwlock wrapper around InputData
impl InputState
{
    pub fn new()
        -> InputState
    {
        InputState {0: RwLock::new(InputData::new())}
    }

    pub fn pressed(&self, key: &str)
        -> bool
    {
        self.0.read().unwrap().pressed(key)
    }

    pub fn update_pressed(&self, presskey : &PressKey, pressed : bool, event_queue : &mut KeyEventQueue)
    {
        self.0.write().unwrap().update_pressed(presskey, pressed, event_queue);
    }
    // pub fn add_event(&mu)
}

macro_rules! set {
    ( $( $x:expr ),* ) => {  // Match zero or more comma delimited items
        {
            use std::collections::HashSet;
            let mut temp_set = HashSet::new();  // Create a mutable HashSet
            $(
                temp_set.insert($x); // Insert each item matched into the HashSet
            )*
            temp_set // Return the populated HashSet
        }
    };
}

impl InputData
{
    pub fn new()
        -> InputData
    {
        let mut input_data = 
            InputData {
                map_press: HashMap::new(), map_key: HashMap::new()};

        input_data.add_keys("forward", set!(18, 72));
        input_data.add("interact_1", set!(PressKey::MouseButton(1)));
        input_data.add("interact_2", set!(PressKey::MouseButton(3)));
        input_data.add_keys("backward", set!(32, 80));
        input_data.add_keys("left", set!(31, 75));
        input_data.add_keys("right", set!(33, 77));
        input_data.add_keys("high", set!(57));
        input_data.add_keys("low", set!(42));

        input_data
    }

    pub fn update_pressed(&mut self, 
        presskey : &PressKey, pressed : bool, event_queue : &mut KeyEventQueue)
    {
        // A list of cloned entries related to the presskey 
        // they will be used to detect changed values
        let related_key_prior_states : Vec<(String, bool)> =
            self.map_key.iter()
            .filter(|(_,v)| v.contains(presskey))
            .map(|(k,_)| (k.clone(), self.pressed(k.clone().as_str())))
            .collect();

        let presskey_changed;

        // The press value is written into the map (if the presskey exists)
        if let Some(mut_val) =  self.map_press.get_mut(&presskey)
        {
            presskey_changed = *mut_val != pressed;

            *mut_val = pressed;
        }
        else
        {
            return
        }

        if presskey_changed
        {
            // All associated key strings that have been affected by the mutation
            let affected_keys : HashSet<String> = 
                related_key_prior_states.into_iter()
                .filter(|(k, v)| *v != self.pressed(k)).map(|(k, _)| k).collect();
            
            event_queue.queue_key_events(&affected_keys, pressed);
            // for queue in queues
            // {
            //     queue
            // }

            // for event in &self.event_handlers
            // {
            //     let event_keys_affected : HashSet<String> = 
            //         affected_keys.intersection(&event.0)
            //         .cloned().collect();
                
            //     if event_keys_affected.len() > 0
            //     {
            //         self.key_event_queue.push_front(
            //             QueueEvent {
            //                 event_handler : event.1.clone(),
            //                 keys_affected : event_keys_affected,
            //                 pressed
            //             }
            //         );
            //     }
            // }

            // for (key, prior_value) in related_key_prior_states
            // {
            //     if prior_value != self.pressed(key.as_str())
            //     {
            //         for (possible_keys, event) in self.event_handlers
            //         {
            //             if possible_keys.contains(&key)
            //             {
            //                 event_handler_affected_set.insert();
            //             }
            //             // event.possible_keys().
            //         }
            //         // self.key_event_queue.push_front(key);
            //     }
            // }
        }
    }

    fn add_keys(&mut self, key: &str, scancodes: HashSet<usize>)
    {
        self.add(key, scancodes.into_iter().map(|code| PressKey::KeyScancode(code)).collect());
    }

    fn add(&mut self, key : &str, new_presskeys : HashSet<PressKey>)
    {
        // Inserts the keys into the press map (assuming they haven't been put in already)
        new_presskeys.iter().cloned()
        .for_each(|pk| {self.map_press.insert(pk, false);});

        // the set of presskeys will either be union-ed with a previous set
        // or it will be added as an entry along with the key
        if let Some(presskey_set) = self.map_key.get_mut(key)
        {
            presskey_set.extend(new_presskeys.into_iter());
        }
        else
        {
            self.map_key.insert(String::from(key), new_presskeys);
        }
    }


    pub fn pressed(&self, key : &str)
        -> bool
    {
        self.map_key
        .get(&String::from(key)).expect("Invalid input Key!").iter()
        .any(|pk| self.map_press[pk])
    }
}