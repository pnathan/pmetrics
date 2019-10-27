# A synchronized queue class
from queue import Queue, Full, Empty
import json

class pmetrics(object):
    """
    threadsafe class to add pmetrics events and measurements to.
    """


    __slots__ = ['events', 'measures']
    def __init__(self):
        self.events = Queue()
        self.measures = Queue()


    # we do not raise on error. it is strictly up to the caller to
    # determine what to do if an error occurs; unthoughtful use should
    # not crash the program.
    #
    # note that we do not store a fully discriminated object in a
    # single queue. this will (i) reduce the redundancy in
    # memory and (ii) reduce contention in locks.

    def add_event(self, name, struct):
        try:
            o = [name, struct]
            self.events.put_nowait(o)
        except Full as e:
            return False, e
        return True

    def add_measure(self, name, measure, struct):
        try:
            self.measure.put_nowait((name, measure, struct))
        except Full as e:
            return False, e
        return True

    def drain_and_get(self):
        """
        Drains the queues and returns the result as a single list.
        """
        # note the whole memory usage is constant, while the pmetrics
        # class drops to near-zero.
        result = []
        try:
            while True:
                o = self.events.get_nowait()
                name = o[0]
                struct = o[1]
                result.append( { 'E' : { 'name': name,
                                         'dict': struct }})
        except Empty:
            pass

        try:
            while True:
                item = self.measures.get_nowait()
                result.append( { 'M' : { 'name': item[0],
                                         'measurement': item[1],
                                         'dict': item[2] }})
        except Empty:
            pass

        return result

    def to_json(self):

        r = self.drain_and_get()
        # because indents help readability.
        return json.dumps(r, indent=1)

PMETRICS = pmetrics()
